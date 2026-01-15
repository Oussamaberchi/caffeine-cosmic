use crate::backend::CaffeineBackend;
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
    ) -> zbus::Result<()>; // Client side uses standard Result

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
            "D-Bus Request: SetState(active={}, idx={})",
            active, selection_idx
        );

        let new_state = if active {
            let selection = match selection_idx {
                0 => TimerSelection::Infinity,
                1 => TimerSelection::OneHour,
                2 => TimerSelection::TwoHours,
                _ => TimerSelection::Manual,
            };

            let manual_u64 = if manual_mins > 0 {
                Some(manual_mins as u64)
            } else {
                None
            };
            let duration = selection.duration_secs(manual_u64);

            let expiry_ts = duration.map(|d| {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(std::time::Duration::from_secs(0))
                    .as_secs()
                    + d
            });

            let reason = match selection {
                TimerSelection::Infinity => "User enabled infinity caffeine mode".to_string(),
                TimerSelection::OneHour => "User enabled 1-hour caffeine timer".to_string(),
                TimerSelection::TwoHours => "User enabled 2-hour caffeine timer".to_string(),
                TimerSelection::Manual => {
                    format!("User enabled {}-minute caffeine timer", manual_mins)
                }
            };

            if let Err(e) = self.backend.inhibit(&reason).await {
                error!("Failed to inhibit via D-Bus: {}", e);
                notify::notify_error(&e);
                return Ok(());
            }

            notify::notify_enabled();
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
