pub mod app_state;
pub mod commands;
pub mod dto;
pub mod log_event;
pub mod receiver_task;
pub mod settings;
pub mod updater;

use app_state::AppState;
use tauri::{Manager, RunEvent, WindowEvent};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if let Some(state) = window.try_state::<AppState>() {
                    let exit_allowed = state
                        .receiver()
                        .lock()
                        .map(|mut receiver| receiver.ordinary_exit_requested())
                        .unwrap_or(false);
                    if !exit_allowed {
                        api.prevent_close();
                    }
                }
            }
        })
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let settings = match settings::load_settings(&config_dir) {
                Ok(settings) => settings,
                Err(error) => {
                    eprintln!(
                        "failed to load settings; automatic update downloads are disabled: {error}"
                    );
                    settings::AppSettings {
                        auto_download_updates: false,
                        ..settings::AppSettings::default()
                    }
                }
            };
            let should_start_receiver = settings.start_receiver_when_app_opens;

            app.manage(AppState::new(
                settings,
                app.package_info().version.to_string(),
            ));
            let state = app.state::<AppState>();
            commands::emit_initial_state(app.handle(), state.inner());

            if should_start_receiver {
                let initial_settings = state
                    .settings()
                    .lock()
                    .ok()
                    .map(|settings| settings.clone());
                if let (Some(settings), Ok(mut receiver)) =
                    (initial_settings, state.receiver().lock())
                {
                    let _ = receiver.start(app.handle().clone(), settings);
                }
            }

            updater::start_background_check(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::set_packet_logging_enabled,
            commands::get_runtime_status,
            commands::start_receiver,
            commands::stop_receiver,
            commands::restart_receiver,
            updater::get_update_snapshot,
            updater::check_for_update,
            updater::download_update,
            updater::defer_update,
            updater::install_update,
            updater::set_auto_download_updates,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build DS Controller app")
        .run(|app, event| {
            if let RunEvent::ExitRequested { code, api, .. } = event {
                if code == Some(tauri::RESTART_EXIT_CODE) {
                    return;
                }
                if let Some(state) = app.try_state::<AppState>() {
                    let exit_allowed = state
                        .receiver()
                        .lock()
                        .map(|mut receiver| receiver.ordinary_exit_requested())
                        .unwrap_or(false);
                    if !exit_allowed {
                        api.prevent_exit();
                    }
                }
            }
        });
}
