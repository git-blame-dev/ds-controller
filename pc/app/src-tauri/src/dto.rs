use serde::{Deserialize, Serialize};

use crate::receiver_task::{ReceiverStatus, RuntimeStatus, VigemStatus};
use crate::settings::AppSettings;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsDto {
    pub port: u16,
    pub start_receiver_when_app_opens: bool,
    pub lock_to_first_sender: bool,
    pub packet_logging_enabled: bool,
}

impl From<AppSettings> for AppSettingsDto {
    fn from(settings: AppSettings) -> Self {
        Self {
            port: settings.port,
            start_receiver_when_app_opens: settings.start_receiver_when_app_opens,
            lock_to_first_sender: settings.lock_to_first_sender,
            packet_logging_enabled: settings.packet_logging_enabled,
        }
    }
}

impl From<AppSettingsDto> for AppSettings {
    fn from(settings: AppSettingsDto) -> Self {
        Self {
            port: settings.port,
            start_receiver_when_app_opens: settings.start_receiver_when_app_opens,
            lock_to_first_sender: settings.lock_to_first_sender,
            packet_logging_enabled: settings.packet_logging_enabled,
            timeout_ms: AppSettings::default().timeout_ms,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatusDto {
    pub receiver: ReceiverStatusDto,
    pub vi_gem: VigemStatusDto,
    pub pressed_buttons: Vec<String>,
    pub packet_count: u64,
    pub last_packet_at: Option<String>,
}

impl From<RuntimeStatus> for RuntimeStatusDto {
    fn from(status: RuntimeStatus) -> Self {
        Self {
            receiver: ReceiverStatusDto::from(status.receiver),
            vi_gem: VigemStatusDto::from(status.vigem),
            pressed_buttons: status.pressed_buttons,
            packet_count: status.packet_count,
            last_packet_at: status.last_packet_at,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ReceiverStatusDto {
    Idle,
    Starting,
    Running {
        bound_address: String,
        locked_sender: Option<String>,
    },
    Stopping,
    Error {
        message: String,
    },
}

impl From<ReceiverStatus> for ReceiverStatusDto {
    fn from(status: ReceiverStatus) -> Self {
        match status {
            ReceiverStatus::Idle => Self::Idle,
            ReceiverStatus::Starting => Self::Starting,
            ReceiverStatus::Running {
                bound_address,
                locked_sender,
            } => Self::Running {
                bound_address,
                locked_sender,
            },
            ReceiverStatus::Stopping => Self::Stopping,
            ReceiverStatus::Error(message) => Self::Error { message },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum VigemStatusDto {
    Unknown,
    Ready,
    Error { message: String },
}

impl From<VigemStatus> for VigemStatusDto {
    fn from(status: VigemStatus) -> Self {
        match status {
            VigemStatus::Unknown => Self::Unknown,
            VigemStatus::Ready => Self::Ready,
            VigemStatus::Error(message) => Self::Error { message },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandErrorDto {
    pub code: String,
    pub message: String,
}

impl CommandErrorDto {
    pub fn not_implemented(command_name: &str) -> Self {
        Self {
            code: "notImplemented".to_owned(),
            message: format!("{command_name} is not implemented yet"),
        }
    }

    pub fn state_unavailable() -> Self {
        Self {
            code: "stateUnavailable".to_owned(),
            message: "application state is unavailable".to_owned(),
        }
    }

    pub fn invalid_settings(message: impl Into<String>) -> Self {
        Self {
            code: "invalidSettings".to_owned(),
            message: message.into(),
        }
    }

    pub fn receiver_error(message: impl Into<String>) -> Self {
        Self {
            code: "receiverError".to_owned(),
            message: message.into(),
        }
    }
}
