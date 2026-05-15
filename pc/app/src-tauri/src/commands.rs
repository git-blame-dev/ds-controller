use tauri::{AppHandle, Emitter, Manager, State};

use crate::app_state::AppState;
use crate::dto::{AppSettingsDto, CommandErrorDto, RuntimeStatusDto};
use crate::receiver_task::{LOG_EVENT, STATUS_EVENT};
use crate::settings::{self, AppSettings};

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppSettingsDto, CommandErrorDto> {
    let settings = state
        .settings()
        .lock()
        .map_err(|_| CommandErrorDto::state_unavailable())?;

    Ok(AppSettingsDto::from(settings.clone()))
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: AppSettingsDto,
) -> Result<AppSettingsDto, CommandErrorDto> {
    let mut settings = AppSettings::from(settings);
    settings
        .validate()
        .map_err(CommandErrorDto::invalid_settings)?;

    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|error| CommandErrorDto::invalid_settings(error.to_string()))?;

    {
        let mut current_settings = state
            .settings()
            .lock()
            .map_err(|_| CommandErrorDto::state_unavailable())?;
        settings.packet_logging_enabled = current_settings.packet_logging_enabled;
        settings::save_settings(&config_dir, &settings)
            .map_err(|error| CommandErrorDto::invalid_settings(error.to_string()))?;
        *current_settings = settings.clone();
    }

    if let Ok(receiver) = state.receiver().lock() {
        receiver.set_packet_logging_enabled(settings.packet_logging_enabled);
    }

    let dto = AppSettingsDto::from(settings);
    let _ = app.emit("settings://changed", dto.clone());

    Ok(dto)
}

#[tauri::command]
pub fn set_packet_logging_enabled(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<AppSettingsDto, CommandErrorDto> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|error| CommandErrorDto::invalid_settings(error.to_string()))?;

    let settings = {
        let mut current_settings = state
            .settings()
            .lock()
            .map_err(|_| CommandErrorDto::state_unavailable())?;
        let previous_packet_logging_enabled = current_settings.packet_logging_enabled;
        let mut settings = current_settings.clone();
        settings.packet_logging_enabled = enabled;

        if let Ok(receiver) = state.receiver().lock() {
            receiver.set_packet_logging_enabled(enabled);
        }

        if let Err(error) = settings::save_settings(&config_dir, &settings) {
            if let Ok(receiver) = state.receiver().lock() {
                receiver.set_packet_logging_enabled(previous_packet_logging_enabled);
            }
            return Err(CommandErrorDto::invalid_settings(error.to_string()));
        }

        *current_settings = settings.clone();
        settings
    };

    Ok(AppSettingsDto::from(settings))
}

#[tauri::command]
pub fn get_runtime_status(state: State<'_, AppState>) -> Result<RuntimeStatusDto, CommandErrorDto> {
    let receiver = state
        .receiver()
        .lock()
        .map_err(|_| CommandErrorDto::state_unavailable())?;

    Ok(RuntimeStatusDto::from(receiver.status()))
}

#[tauri::command]
pub fn start_receiver(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<RuntimeStatusDto, CommandErrorDto> {
    let settings = state
        .settings()
        .lock()
        .map_err(|_| CommandErrorDto::state_unavailable())?
        .clone();

    let mut receiver = state
        .receiver()
        .lock()
        .map_err(|_| CommandErrorDto::state_unavailable())?;

    Ok(RuntimeStatusDto::from(receiver.start(app, settings)))
}

#[tauri::command]
pub fn stop_receiver(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<RuntimeStatusDto, CommandErrorDto> {
    let mut receiver = state
        .receiver()
        .lock()
        .map_err(|_| CommandErrorDto::state_unavailable())?;

    Ok(RuntimeStatusDto::from(receiver.stop(&app)))
}

#[tauri::command]
pub fn restart_receiver(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<RuntimeStatusDto, CommandErrorDto> {
    let settings = state
        .settings()
        .lock()
        .map_err(|_| CommandErrorDto::state_unavailable())?
        .clone();

    let mut receiver = state
        .receiver()
        .lock()
        .map_err(|_| CommandErrorDto::state_unavailable())?;

    Ok(RuntimeStatusDto::from(receiver.restart(app, settings)))
}

pub fn emit_initial_state(app: &AppHandle, state: &AppState) {
    if let Ok(receiver) = state.receiver().lock() {
        let _ = app.emit(STATUS_EVENT, RuntimeStatusDto::from(receiver.status()));
    }

    if let Ok(settings) = state.settings().lock() {
        let _ = app.emit("settings://changed", AppSettingsDto::from(settings.clone()));
    }

    let _ = app.emit(
        LOG_EVENT,
        crate::log_event::LogEvent::new(crate::log_event::LogLevel::Info, "DS Controller ready"),
    );
}
