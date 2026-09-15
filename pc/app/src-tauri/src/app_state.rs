use std::sync::{Arc, Mutex};

use tauri_plugin_updater::Update;

use crate::receiver_task::ReceiverController;
use crate::settings::AppSettings;
use crate::updater::UpdateSession;

pub struct AppState {
    settings: Mutex<AppSettings>,
    receiver: Arc<Mutex<ReceiverController>>,
    updater: Arc<Mutex<UpdateSession<Update>>>,
}

impl AppState {
    pub fn new(settings: AppSettings, current_version: String) -> Self {
        let auto_download_updates = settings.auto_download_updates;
        Self {
            settings: Mutex::new(settings),
            receiver: Arc::new(Mutex::new(ReceiverController::default())),
            updater: Arc::new(Mutex::new(UpdateSession::new(
                current_version,
                auto_download_updates,
            ))),
        }
    }

    pub fn settings(&self) -> &Mutex<AppSettings> {
        &self.settings
    }

    pub fn receiver(&self) -> &Arc<Mutex<ReceiverController>> {
        &self.receiver
    }

    pub fn updater(&self) -> &Arc<Mutex<UpdateSession<Update>>> {
        &self.updater
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(AppSettings::default(), env!("CARGO_PKG_VERSION").to_owned())
    }
}
