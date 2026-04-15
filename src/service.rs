use crate::backend::CaffeineBackend;
use crate::config::{CaffeineConfig, InhibitMode};
use crate::notify;
use crate::state::{CaffeineState, TimerSelection};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info};
use zbus::{interface, object_server::SignalEmitter, proxy};

pub const DBUS_NAME: &str = "com.github.oussama_berchi.cosmic_caffeine";
pub const DBUS_PATH: &str = "/com/github/oussama_berchi/cosmic_caffeine";
pub const DBUS_INTERFACE: &str = "com.github.oussama_berchi.cosmic_caffeine.Manager";

#[derive(Clone)]
pub struct CaffeineService {
    backend: CaffeineBackend,
    state: Arc<Mutex<CaffeineState>>,
}

impl CaffeineService {
    pub fn new(backend: CaffeineBackend, state: Arc<Mutex<CaffeineState>>) -> Self {
        Self { backend, state }
    }
}

fn idx_to_selection(idx: u32) -> TimerSelection {
    match idx {
        0 => TimerSelection::Infinity,
        1 => TimerSelection::FiveMins,
        2 => TimerSelection::TenMins,
        3 => TimerSelection::ThirtyMins,
        4 => TimerSelection::OneHour,
        5 => TimerSelection::TwoHours,
        6 => TimerSelection::ThreeHours,
        7 => TimerSelection::FourHours,
        _ => TimerSelection::Manual,
    }
}

#[proxy(
    interface = "com.github.oussama_berchi.cosmic_caffeine.Manager",
    default_service = "com.github.oussama_berchi.cosmic_caffeine",
    default_path = "/com/github/oussama_berchi/cosmic_caffeine"
)]
pub trait CaffeineManager {
    async fn set_state(
        &self,
        active: bool,
        selection_idx: u32,
        manual_mins: u32,
    ) -> zbus::Result<()>;

    async fn get_state(&self) -> zbus::Result<CaffeineState>;
}

#[interface(name = "com.github.oussama_berchi.cosmic_caffeine.Manager")]
impl CaffeineService {
    async fn set_state(
        &mut self,
        active: bool,
        selection_idx: u32,
        manual_mins: u32,
        #[zbus(signal_emitter)] ctxt: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        info!(
            "D-Bus Request: SetState(active={}, idx={}, mins={})",
            active, selection_idx, manual_mins
        );

        let config = CaffeineConfig::load();

        let new_state = if active {
            let selection = idx_to_selection(selection_idx);

            let manual_hours = if selection == TimerSelection::Manual {
                config.manual_hours
            } else {
                0
            };
            let duration = selection.duration_secs(Some(manual_mins as u64), Some(manual_hours as u64));

            let expiry_ts = duration.map(|d| {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(std::time::Duration::from_secs(0))
                    .as_secs()
                    + d
            });

            let reason = format!("Caffeine enabled: {}", selection.label());

            let inhibit_mode = config.inhibit_mode;
            if let Err(e) = self.backend.inhibit(&reason, inhibit_mode).await {
                error!("Failed to inhibit via D-Bus: {}", e);
                notify::notify_error(&e);
                return Ok(());
            }

            if let Some(secs) = duration {
                let mins = (secs / 60) as u32;
                if mins > 0 {
                    notify::notify_enabled_with_time(&format!("{} {}", mins, fl!("notification-minutes-left")));
                }
            } else {
                notify::notify_enabled();
            }

            CaffeineState::active(selection, expiry_ts)
        } else {
            if let Err(e) = self.backend.uninhibit().await {
                error!("Failed to uninhibit via D-Bus: {}", e);
                notify::notify_error(&e);
            }
            notify::notify_disabled();
            CaffeineState::inactive()
        };

        {
            if let Ok(mut lock) = self.state.lock() {
                *lock = new_state;
            } else {
                error!("Failed to acquire lock on state");
            }
        }

        if let Err(e) = ctxt.emit(DBUS_INTERFACE, "StateChanged", &new_state).await {
            error!("Failed to emit signal: {}", e);
        }
        Ok(())
    }

    async fn get_state(&self) -> CaffeineState {
        if let Ok(lock) = self.state.lock() {
            *lock
        } else {
            error!("Failed to acquire lock on state");
            CaffeineState::inactive()
        }
    }
}