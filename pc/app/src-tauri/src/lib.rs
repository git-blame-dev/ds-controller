pub mod app_state;
pub mod commands;
pub mod dto;
pub mod log_event;
pub mod receiver_task;
pub mod settings;

use app_state::AppState;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let settings = match settings::load_settings(&config_dir) {
                Ok(settings) => settings,
                Err(error) => {
                    eprintln!("failed to load settings, using defaults: {error}");
                    settings::AppSettings::default()
                }
            };
            let should_start_receiver = settings.start_receiver_when_app_opens;

            app.manage(AppState::new(settings));
            let state = app.state::<AppState>();
            commands::emit_initial_state(app.handle(), state.inner());

            if should_start_receiver {
                if let (Ok(settings), Ok(mut receiver)) =
                    (state.settings().lock(), state.receiver().lock())
                {
                    receiver.start(app.handle().clone(), settings.clone());
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::get_runtime_status,
            commands::start_receiver,
            commands::stop_receiver,
            commands::restart_receiver,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run DS Controller app");
}
