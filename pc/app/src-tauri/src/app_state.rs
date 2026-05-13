use std::sync::Mutex;

use crate::receiver_task::ReceiverController;
use crate::settings::AppSettings;

pub struct AppState {
    settings: Mutex<AppSettings>,
    receiver: Mutex<ReceiverController>,
}

impl AppState {
    pub fn new(settings: AppSettings) -> Self {
        Self {
            settings: Mutex::new(settings),
            receiver: Mutex::new(ReceiverController::default()),
        }
    }

    pub fn settings(&self) -> &Mutex<AppSettings> {
        &self.settings
    }

    pub fn receiver(&self) -> &Mutex<ReceiverController> {
        &self.receiver
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(AppSettings::default())
    }
}
