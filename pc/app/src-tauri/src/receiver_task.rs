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
        locked_sender: Option<String>,
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
        fixed_sender: None,
        accept_first_sender: settings.lock_to_first_sender,
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
                locked_sender: None,
            },
            vigem: VigemStatus::Ready,
            pressed_buttons: Vec::new(),
            packet_count: 0,
            last_packet_at: None,
        },
    );
    emit_log(&app, LogLevel::Info, "virtual controller ready");

    let mut packet_count = 0;

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
                if let Err(error) = backend.update(output) {
                    emit_log(
                        &app,
                        LogLevel::Error,
                        format!("controller update failed: {error}"),
                    );
                }

                let pressed_buttons = button_names(state);
                set_status(
                    &app,
                    &status,
                    RuntimeStatus {
                        receiver: ReceiverStatus::Running {
                            bound_address: bind_addr.to_string(),
                            locked_sender: Some(sender.to_string()),
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
                if let Err(error) = backend.neutral() {
                    emit_log(
                        &app,
                        LogLevel::Error,
                        format!("neutral controller update failed: {error}"),
                    );
                }
                set_status(
                    &app,
                    &status,
                    RuntimeStatus {
                        receiver: ReceiverStatus::Running {
                            bound_address: bind_addr.to_string(),
                            locked_sender: None,
                        },
                        vigem: VigemStatus::Ready,
                        pressed_buttons: Vec::new(),
                        packet_count,
                        last_packet_at: None,
                    },
                );
                emit_log(&app, LogLevel::Info, "receiver timeout: release all inputs");
            }
            Err(error) => {
                emit_log(&app, LogLevel::Error, format!("receiver error: {error}"));
            }
        }
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
