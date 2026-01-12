use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use ashpd::desktop::inhibit::{InhibitFlags, InhibitProxy};
use ashpd::desktop::Request;
use ashpd::enumflags2::{make_bitflags, BitFlags};

const INHIBIT_FLAGS: BitFlags<InhibitFlags> = make_bitflags!(InhibitFlags::{Idle});

#[derive(Clone)]
pub struct CaffeineBackend {
    state: Arc<Mutex<BackendState>>,
}

#[derive(Debug, Default)]
struct BackendState {
    inhibit_handle: Option<Request<()>>,
}

impl CaffeineBackend {
    pub fn new() -> Self {
        info!("CaffeineBackend initialized");
        Self {
            state: Arc::new(Mutex::new(BackendState::default())),
        }
    }

    pub async fn inhibit(&self, reason: &str) -> Result<(), String> {
        info!(
            "Attempting to inhibit idle via XDG portal, reason: {}",
            reason
        );

        {
            let state = self.state.lock().await;
            if state.inhibit_handle.is_some() {
                warn!("Already inhibiting, skipping duplicate request");
                return Ok(());
            }
        }

        let proxy = InhibitProxy::new().await.map_err(|e| {
            let msg = format!("Failed to create InhibitProxy: {}", e);
            error!("{}", msg);
            msg
        })?;

        debug!("InhibitProxy created successfully");

        let request = proxy
            .inhibit(None, INHIBIT_FLAGS, reason)
            .await
            .map_err(|e| {
                let msg = format!("Failed to call inhibit: {}", e);
                error!("{}", msg);
                debug!("D-Bus error details: {:?}", e);
                msg
            })?;

        debug!("Inhibit request successful, handle obtained");
        info!("Screen idle inhibition activated successfully");

        let mut state = self.state.lock().await;
        state.inhibit_handle = Some(request);

        Ok(())
    }

    pub async fn uninhibit(&self) -> Result<(), String> {
        info!("Attempting to uninhibit (release idle lock)");

        let mut state = self.state.lock().await;

        if let Some(handle) = state.inhibit_handle.take() {
            debug!("Closing inhibit handle");
            handle.close().await.map_err(|e| {
                let msg = format!("Failed to close inhibit handle: {}", e);
                error!("{}", msg);
                debug!("D-Bus error details: {:?}", e);
                msg
            })?;

            info!("Screen idle inhibition released successfully");
            Ok(())
        } else {
            warn!("No active inhibition to release");
            Ok(())
        }
    }
}

impl Drop for CaffeineBackend {
    fn drop(&mut self) {
        debug!("CaffeineBackend dropped");
    }
}
