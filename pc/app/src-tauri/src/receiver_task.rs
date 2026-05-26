use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use ds_controller_receiver::backend::create_backend;
use ds_controller_receiver::mapping::map_ds_to_xbox;
use ds_controller_receiver::protocol::{Buttons, ControllerState};
use ds_controller_receiver::receiver::{Receiver, ReceiverConfig, ReceiverEvent};
use tauri::{AppHandle, Emitter};

use crate::dto::RuntimeStatusDto;
use crate::log_event::{LogEvent, LogLevel};
use crate::settings::AppSettings;

pub const STATUS_EVENT: &str = "receiver://status";
pub const LOG_EVENT: &str = "receiver://log";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReceiverStatus {
    Idle,
    Starting,
    Running {
        bound_address: String,
        last_sender: Option<String>,
    },
    Stopping,
    Error(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VigemStatus {
    Unknown,
    Ready,
    Error(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeStatus {
    pub receiver: ReceiverStatus,
    pub vigem: VigemStatus,
    pub pressed_buttons: Vec<String>,
    pub packet_count: u64,
    pub last_packet_at: Option<String>,
}

impl Default for RuntimeStatus {
    fn default() -> Self {
        Self {
            receiver: ReceiverStatus::Idle,
            vigem: VigemStatus::Unknown,
            pressed_buttons: Vec::new(),
            packet_count: 0,
            last_packet_at: None,
        }
    }
}

pub struct ReceiverController {
    status: Arc<Mutex<RuntimeStatus>>,
    stop_tx: Option<mpsc::Sender<()>>,
    join_handle: Option<JoinHandle<()>>,
    packet_logging_enabled: Arc<AtomicBool>,
}

impl ReceiverController {
    pub fn status(&self) -> RuntimeStatus {
        self.status
            .lock()
            .map(|status| status.clone())
            .unwrap_or_else(|_| RuntimeStatus {
                receiver: ReceiverStatus::Error("receiver status is unavailable".to_owned()),
                vigem: VigemStatus::Unknown,
                pressed_buttons: Vec::new(),
                packet_count: 0,
                last_packet_at: None,
            })
    }

    pub fn start(&mut self, app: AppHandle, settings: AppSettings) -> RuntimeStatus {
        self.packet_logging_enabled
            .store(settings.packet_logging_enabled, Ordering::Relaxed);
        self.reap_finished_worker(&app);

        if self.stop_tx.is_some() {
            return self.status();
        }

        let bind_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, settings.port));
        let (stop_tx, stop_rx) = mpsc::channel();
        let status = Arc::clone(&self.status);
        let packet_logging_enabled = Arc::clone(&self.packet_logging_enabled);

        set_status(
            &app,
            &status,
            RuntimeStatus {
                receiver: ReceiverStatus::Starting,
                vigem: VigemStatus::Unknown,
                pressed_buttons: Vec::new(),
                packet_count: 0,
                last_packet_at: None,
            },
        );
        emit_log(
            &app,
            LogLevel::Info,
            format!("starting receiver on {bind_addr}"),
        );

        let join_handle = thread::spawn(move || {
            run_receiver_worker(
                app,
                status,
                packet_logging_enabled,
                stop_rx,
                bind_addr,
                settings,
            );
        });

        self.stop_tx = Some(stop_tx);
        self.join_handle = Some(join_handle);
        self.status()
    }

    pub fn stop(&mut self, app: &AppHandle) -> RuntimeStatus {
        if let Some(stop_tx) = self.stop_tx.take() {
            set_status(
                app,
                &self.status,
                RuntimeStatus {
                    receiver: ReceiverStatus::Stopping,
                    ..self.status()
                },
            );
            let _ = stop_tx.send(());
        }

        if let Some(join_handle) = self.join_handle.take() {
            if join_handle.join().is_err() {
                emit_log(app, LogLevel::Error, "receiver worker panicked");
            }
        }

        set_status(app, &self.status, RuntimeStatus::default());
        emit_log(app, LogLevel::Info, "receiver stopped");
        self.status()
    }

    pub fn restart(&mut self, app: AppHandle, settings: AppSettings) -> RuntimeStatus {
        self.stop(&app);
        self.start(app, settings)
    }

    pub fn set_packet_logging_enabled(&self, enabled: bool) {
        self.packet_logging_enabled
            .store(enabled, Ordering::Relaxed);
    }

    fn reap_finished_worker(&mut self, app: &AppHandle) {
        let worker_finished = self
            .join_handle
            .as_ref()
            .map(|join_handle| join_handle.is_finished())
            .unwrap_or(false);

        if !worker_finished {
            return;
        }

        self.stop_tx = None;
        if let Some(join_handle) = self.join_handle.take() {
            if join_handle.join().is_err() {
                emit_log(app, LogLevel::Error, "receiver worker panicked");
            }
        }
    }
}

impl Default for ReceiverController {
    fn default() -> Self {
        Self {
            status: Arc::new(Mutex::new(RuntimeStatus::default())),
            stop_tx: None,
            join_handle: None,
            packet_logging_enabled: Arc::new(AtomicBool::new(false)),
        }
    }
}

fn run_receiver_worker(
    app: AppHandle,
    status: Arc<Mutex<RuntimeStatus>>,
    packet_logging_enabled: Arc<AtomicBool>,
    stop_rx: mpsc::Receiver<()>,
    bind_addr: SocketAddr,
    settings: AppSettings,
) {
    let config = ReceiverConfig {
        bind_addr,
        timeout: Duration::from_millis(settings.timeout_ms),
    };

    let mut receiver = match Receiver::bind(config) {
        Ok(receiver) => receiver,
        Err(error) => {
            let message = format!("failed to bind UDP receiver: {error}");
            emit_log(&app, LogLevel::Error, &message);
            set_status(
                &app,
                &status,
                RuntimeStatus {
                    receiver: ReceiverStatus::Error(message),
                    vigem: VigemStatus::Unknown,
                    pressed_buttons: Vec::new(),
                    packet_count: 0,
                    last_packet_at: None,
                },
            );
            return;
        }
    };

    emit_log(&app, LogLevel::Info, format!("listening on {bind_addr}"));

    let mut backend = match create_backend(false) {
        Ok(backend) => backend,
        Err(error) => {
            let message = format!("failed to initialize controller backend: {error}");
            emit_log(&app, LogLevel::Error, &message);
            set_status(
                &app,
                &status,
                RuntimeStatus {
                    receiver: ReceiverStatus::Error(message.clone()),
                    vigem: VigemStatus::Error(message),
                    pressed_buttons: Vec::new(),
                    packet_count: 0,
                    last_packet_at: None,
                },
            );
            return;
        }
    };

    set_status(
        &app,
        &status,
        RuntimeStatus {
            receiver: ReceiverStatus::Running {
                bound_address: bind_addr.to_string(),
                last_sender: None,
            },
            vigem: VigemStatus::Ready,
            pressed_buttons: Vec::new(),
            packet_count: 0,
            last_packet_at: None,
        },
    );
    emit_log(&app, LogLevel::Info, "virtual controller ready");

    let mut packet_count = 0;
    let mut receiver_session_state = ReceiverSessionState::default();

    loop {
        if stop_rx.try_recv().is_ok() {
            if let Err(error) = backend.neutral() {
                emit_log(
                    &app,
                    LogLevel::Error,
                    format!("neutral controller update failed: {error}"),
                );
            }
            return;
        }

        match receiver.next_event() {
            Ok(ReceiverEvent::State { sender, state }) => {
                let output = map_ds_to_xbox(state);
                packet_count += 1;
                match backend.update(output) {
                    Ok(()) => {
                        receiver_session_state.record_update(state.buttons);
                    }
                    Err(error) => {
                        emit_log(
                            &app,
                            LogLevel::Error,
                            format!("controller update failed: {error}"),
                        );
                    }
                }

                let pressed_buttons = button_names(state);
                set_status(
                    &app,
                    &status,
                    RuntimeStatus {
                        receiver: ReceiverStatus::Running {
                            bound_address: bind_addr.to_string(),
                            last_sender: Some(sender.to_string()),
                        },
                        vigem: VigemStatus::Ready,
                        pressed_buttons,
                        packet_count,
                        last_packet_at: Some(now_millis_string()),
                    },
                );

                if packet_logging_enabled.load(Ordering::Relaxed) {
                    emit_log(
                        &app,
                        LogLevel::Packet,
                        format!(
                            "{sender} seq={} ds={} xbox={}",
                            state.sequence, state.buttons, output.buttons
                        ),
                    );
                }
            }
            Ok(ReceiverEvent::Timeout) => {
                let needs_timeout_release = receiver_session_state.needs_timeout_release();
                let mut timeout_release_succeeded = false;

                if needs_timeout_release {
                    match backend.neutral() {
                        Ok(()) => {
                            receiver_session_state.record_neutral();
                            timeout_release_succeeded = true;
                        }
                        Err(error) => {
                            if receiver_session_state.record_timeout_release_error() {
                                emit_log(
                                    &app,
                                    LogLevel::Error,
                                    format!("neutral controller update failed: {error}"),
                                );
                            }
                        }
                    }
                }

                if !receiver_session_state.needs_timeout_status() {
                    continue;
                }

                set_status(
                    &app,
                    &status,
                    RuntimeStatus {
                        receiver: ReceiverStatus::Running {
                            bound_address: bind_addr.to_string(),
                            last_sender: None,
                        },
                        vigem: VigemStatus::Ready,
                        pressed_buttons: Vec::new(),
                        packet_count,
                        last_packet_at: None,
                    },
                );
                receiver_session_state.record_timeout_status();

                if timeout_release_succeeded {
                    emit_log(&app, LogLevel::Info, "receiver timeout: release all inputs");
                }
            }
            Err(error) => {
                emit_log(&app, LogLevel::Error, format!("receiver error: {error}"));
            }
        }
    }
}

#[derive(Default)]
struct ReceiverSessionState {
    has_pressed_inputs: bool,
    timeout_status_reported: bool,
    timeout_release_error_reported: bool,
}

impl ReceiverSessionState {
    fn record_update(&mut self, buttons: Buttons) {
        self.has_pressed_inputs = !buttons.is_empty();
        self.timeout_status_reported = false;
        self.timeout_release_error_reported = false;
    }

    fn record_neutral(&mut self) {
        self.has_pressed_inputs = false;
        self.timeout_release_error_reported = false;
    }

    fn needs_timeout_release(&self) -> bool {
        self.has_pressed_inputs
    }

    fn needs_timeout_status(&self) -> bool {
        !self.timeout_status_reported
    }

    fn record_timeout_status(&mut self) {
        self.timeout_status_reported = true;
    }

    fn record_timeout_release_error(&mut self) -> bool {
        if self.timeout_release_error_reported {
            return false;
        }

        self.timeout_release_error_reported = true;
        true
    }
}

fn set_status(app: &AppHandle, status: &Arc<Mutex<RuntimeStatus>>, next_status: RuntimeStatus) {
    if let Ok(mut status) = status.lock() {
        *status = next_status.clone();
    }

    let _ = app.emit(STATUS_EVENT, RuntimeStatusDto::from(next_status));
}

fn emit_log(app: &AppHandle, level: LogLevel, message: impl Into<String>) {
    let _ = app.emit(LOG_EVENT, LogEvent::new(level, message));
}

fn now_millis_string() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().to_string())
        .unwrap_or_else(|_| "0".to_owned())
}

fn button_names(state: ControllerState) -> Vec<String> {
    let buttons = [
        (Buttons::A, "a"),
        (Buttons::B, "b"),
        (Buttons::X, "x"),
        (Buttons::Y, "y"),
        (Buttons::L, "l"),
        (Buttons::R, "r"),
        (Buttons::START, "start"),
        (Buttons::SELECT, "select"),
        (Buttons::DPAD_UP, "up"),
        (Buttons::DPAD_DOWN, "down"),
        (Buttons::DPAD_LEFT, "left"),
        (Buttons::DPAD_RIGHT, "right"),
    ];

    buttons
        .into_iter()
        .filter(|(button, _name)| state.buttons.contains(*button))
        .map(|(_button, name)| name.to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_backend_state_does_not_need_timeout_release() {
        let state = ReceiverSessionState::default();

        assert!(!state.needs_timeout_release());
    }

    #[test]
    fn pressed_backend_state_needs_timeout_release() {
        let mut state = ReceiverSessionState::default();

        state.record_update(Buttons::A);

        assert!(state.needs_timeout_release());
    }

    #[test]
    fn neutral_update_clears_timeout_release_need() {
        let mut state = ReceiverSessionState::default();
        state.record_update(Buttons::A);

        state.record_update(Buttons::default());

        assert!(!state.needs_timeout_release());
    }

    #[test]
    fn timeout_release_remains_pending_until_neutral_is_recorded() {
        let mut state = ReceiverSessionState::default();
        state.record_update(Buttons::A);

        assert!(state.needs_timeout_release());
    }

    #[test]
    fn timeout_status_does_not_clear_pending_release() {
        let mut state = ReceiverSessionState::default();
        state.record_update(Buttons::A);

        state.record_timeout_status();

        assert!(state.needs_timeout_release());
        assert!(!state.needs_timeout_status());
    }

    #[test]
    fn timeout_release_error_is_reported_once_until_next_update() {
        let mut state = ReceiverSessionState::default();
        state.record_update(Buttons::A);

        assert!(state.record_timeout_release_error());
        assert!(!state.record_timeout_release_error());

        state.record_update(Buttons::A);

        assert!(state.record_timeout_release_error());
    }

    #[test]
    fn successful_timeout_release_clears_release_need() {
        let mut state = ReceiverSessionState::default();
        state.record_update(Buttons::A);

        state.record_neutral();

        assert!(!state.needs_timeout_release());
        assert!(state.record_timeout_release_error());
    }

    #[test]
    fn neutral_backend_state_still_needs_timeout_status_once() {
        let state = ReceiverSessionState::default();

        assert!(state.needs_timeout_status());
    }

    #[test]
    fn timeout_status_is_suppressed_until_next_update() {
        let mut state = ReceiverSessionState::default();

        state.record_timeout_status();

        assert!(!state.needs_timeout_status());

        state.record_update(Buttons::default());

        assert!(state.needs_timeout_status());
    }
}
