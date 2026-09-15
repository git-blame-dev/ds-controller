use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const DEFAULT_PORT: u16 = 26_760;
pub const DEFAULT_TIMEOUT_MS: u64 = 75;
pub const MAX_TIMEOUT_MS: u64 = 1_000;
const LEGACY_DEFAULT_TIMEOUT_MS: u64 = 150;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub port: u16,
    pub start_receiver_when_app_opens: bool,
    pub packet_logging_enabled: bool,
    #[serde(default = "default_auto_download_updates")]
    pub auto_download_updates: bool,
    pub timeout_ms: u64,
}

#[derive(Debug)]
pub enum SettingsError {
    Io(io::Error),
    Serde(serde_json::Error),
}

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "settings I/O error: {error}"),
            Self::Serde(error) => write!(f, "settings format error: {error}"),
        }
    }
}

impl std::error::Error for SettingsError {}

impl From<io::Error> for SettingsError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for SettingsError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serde(error)
    }
}

impl AppSettings {
    pub fn validate(&self) -> Result<(), String> {
        if self.port == 0 {
            return Err("port must be between 1 and 65535".to_owned());
        }

        if !(1..=MAX_TIMEOUT_MS).contains(&self.timeout_ms) {
            return Err("timeout must be between 1 and 1000 milliseconds".to_owned());
        }

        Ok(())
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            start_receiver_when_app_opens: true,
            packet_logging_enabled: false,
            auto_download_updates: true,
            timeout_ms: DEFAULT_TIMEOUT_MS,
        }
    }
}

const fn default_auto_download_updates() -> bool {
    true
}

fn migrate_settings(settings: &mut AppSettings) {
    if settings.timeout_ms == LEGACY_DEFAULT_TIMEOUT_MS {
        settings.timeout_ms = DEFAULT_TIMEOUT_MS;
    }
}

pub fn settings_path(config_dir: &Path) -> PathBuf {
    config_dir.join("settings.json")
}

pub fn load_settings(config_dir: &Path) -> Result<AppSettings, SettingsError> {
    let path = settings_path(config_dir);
    if !path.exists() {
        return Ok(AppSettings::default());
    }

    let mut settings = serde_json::from_slice::<AppSettings>(&fs::read(path)?)?;
    migrate_settings(&mut settings);
    settings.validate().map_err(|message| {
        SettingsError::Serde(serde_json::Error::io(io::Error::new(
            io::ErrorKind::InvalidData,
            message,
        )))
    })?;

    Ok(settings)
}

pub fn save_settings(config_dir: &Path, settings: &AppSettings) -> Result<(), SettingsError> {
    settings.validate().map_err(|message| {
        SettingsError::Serde(serde_json::Error::io(io::Error::new(
            io::ErrorKind::InvalidInput,
            message,
        )))
    })?;

    fs::create_dir_all(config_dir)?;
    fs::write(
        settings_path(config_dir),
        serde_json::to_vec_pretty(settings)?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_planned_default_settings() {
        let settings = AppSettings::default();

        assert_eq!(settings.port, DEFAULT_PORT);
        assert!(settings.start_receiver_when_app_opens);
        assert!(!settings.packet_logging_enabled);
        assert!(settings.auto_download_updates);
        assert_eq!(DEFAULT_TIMEOUT_MS, 75);
        assert_eq!(settings.timeout_ms, DEFAULT_TIMEOUT_MS);
    }

    #[test]
    fn migrates_legacy_default_timeout() {
        let mut settings = AppSettings {
            timeout_ms: LEGACY_DEFAULT_TIMEOUT_MS,
            ..AppSettings::default()
        };

        migrate_settings(&mut settings);

        assert_eq!(settings.timeout_ms, DEFAULT_TIMEOUT_MS);
    }

    #[test]
    fn fresh_and_legacy_valid_settings_enable_automatic_update_downloads() {
        let fresh = AppSettings::default();
        let legacy = serde_json::from_value::<AppSettings>(serde_json::json!({
            "port": DEFAULT_PORT,
            "startReceiverWhenAppOpens": true,
            "packetLoggingEnabled": false,
            "timeoutMs": DEFAULT_TIMEOUT_MS
        }))
        .expect("valid legacy settings should migrate");

        assert!(fresh.auto_download_updates);
        assert!(legacy.auto_download_updates);
    }

    #[test]
    fn rejects_timeout_values_that_can_make_receiver_shutdown_unbounded() {
        let settings = AppSettings {
            timeout_ms: 60_001,
            ..AppSettings::default()
        };

        assert_eq!(
            settings.validate(),
            Err("timeout must be between 1 and 1000 milliseconds".to_owned())
        );
    }
}
