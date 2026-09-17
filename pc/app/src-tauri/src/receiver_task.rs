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
pub enum VirtualControllerStatus {
    Unknown,
    Ready,
    Error(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeStatus {
    pub receiver: ReceiverStatus,
    pub virtual_controller: VirtualControllerStatus,
    pub pressed_buttons: Vec<String>,
    pub packet_count: u64,
    pub last_packet_at: Option<String>,
}

impl Default for RuntimeStatus {
    fn default() -> Self {
        Self {
            receiver: ReceiverStatus::Idle,
            virtual_controller: VirtualControllerStatus::Unknown,
            pressed_buttons: Vec::new(),
            packet_count: 0,
            last_packet_at: None,
        }
    }
}

pub struct ReceiverController {
    status: Arc<Mutex<RuntimeStatus>>,
    stop_tx: Option<mpsc::Sender<()>>,
    join_handle: Option<JoinHandle<WorkerOutcome>>,
    packet_logging_enabled: Arc<AtomicBool>,
    lifecycle: LifecycleState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LifecycleState {
    Normal,
    Stopping,
    InstallReserved,
    ExitAccepted,
    RestartRequired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorkerOutcome {
    Neutral,
    NeutralFailed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StopOutcome {
    pub worker_terminated: bool,
    pub neutral: bool,
}

pub struct ReceiverStopTask {
    join_handle: Option<JoinHandle<WorkerOutcome>>,
}

impl ReceiverStopTask {
    pub fn join(self) -> StopOutcome {
        match self.join_handle {
            Some(join_handle) => match join_handle.join() {
                Ok(outcome) => StopOutcome {
                    worker_terminated: true,
                    neutral: outcome == WorkerOutcome::Neutral,
                },
                Err(_) => StopOutcome {
                    worker_terminated: true,
                    neutral: false,
                },
            },
            None => StopOutcome {
                worker_terminated: true,
                neutral: true,
            },
        }
    }
}

impl StopOutcome {
    pub fn install_ready(self) -> bool {
        self.worker_terminated && self.neutral
    }
}

impl ReceiverController {
    pub fn status(&self) -> RuntimeStatus {
        self.status
            .lock()
            .map(|status| status.clone())
            .unwrap_or_else(|_| RuntimeStatus {
                receiver: ReceiverStatus::Error("receiver status is unavailable".to_owned()),
                virtual_controller: VirtualControllerStatus::Unknown,
                pressed_buttons: Vec::new(),
                packet_count: 0,
                last_packet_at: None,
            })
    }

    pub fn start(
        &mut self,
        app: AppHandle,
        settings: AppSettings,
    ) -> Result<RuntimeStatus, String> {
        if self.lifecycle != LifecycleState::Normal {
            return Err(
                "receiver lifecycle is reserved for update installation or exit".to_owned(),
            );
        }
        self.packet_logging_enabled
            .store(settings.packet_logging_enabled, Ordering::Relaxed);
        self.reap_finished_worker(&app);

        if self.stop_tx.is_some() {
            return Ok(self.status());
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
                virtual_controller: VirtualControllerStatus::Unknown,
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
            )
        });

        self.stop_tx = Some(stop_tx);
        self.join_handle = Some(join_handle);
        Ok(self.status())
    }

    pub fn begin_stop(&mut self, app: &AppHandle) -> Result<ReceiverStopTask, String> {
        if self.lifecycle != LifecycleState::Normal {
            return Err(
                "receiver lifecycle is reserved for update installation or exit".to_owned(),
            );
        }
        self.lifecycle = LifecycleState::Stopping;
        Ok(self.begin_stop_worker(app))
    }

    fn begin_stop_worker(&mut self, app: &AppHandle) -> ReceiverStopTask {
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
        ReceiverStopTask {
            join_handle: self.join_handle.take(),
        }
    }

    pub fn finish_stop(&mut self, app: &AppHandle, outcome: StopOutcome) -> RuntimeStatus {
        if outcome.install_ready() {
            set_status(app, &self.status, RuntimeStatus::default());
            emit_log(app, LogLevel::Info, "receiver stopped");
        } else {
            let message = "receiver stopped without confirmed neutral controller output";
            set_status(
                app,
                &self.status,
                RuntimeStatus {
                    receiver: ReceiverStatus::Error(message.to_owned()),
                    virtual_controller: VirtualControllerStatus::Error(message.to_owned()),
                    pressed_buttons: Vec::new(),
                    packet_count: 0,
                    last_packet_at: None,
                },
            );
            emit_log(app, LogLevel::Error, message);
        }
        if self.lifecycle == LifecycleState::Stopping {
            self.lifecycle = LifecycleState::Normal;
        }
        self.status()
    }

    pub fn reserve_install(&mut self) -> Result<(), String> {
        if self.lifecycle != LifecycleState::Normal {
            return Err(
                "application is already exiting or reserved for update installation".to_owned(),
            );
        }
        self.lifecycle = LifecycleState::InstallReserved;
        Ok(())
    }

    pub fn reserve_install_if_idle(&mut self) -> bool {
        let safely_idle = self.lifecycle == LifecycleState::Normal
            && self.stop_tx.is_none()
            && self.join_handle.is_none()
            && self.status() == RuntimeStatus::default();
        if !safely_idle {
            return false;
        }
        self.lifecycle = LifecycleState::InstallReserved;
        true
    }

    pub fn cancel_install_reservation(&mut self) {
        if self.lifecycle == LifecycleState::InstallReserved {
            self.lifecycle = LifecycleState::Normal;
        }
    }

    pub fn finish_install_preparation_failure(&mut self, outcome: StopOutcome) {
        self.lifecycle = if outcome.worker_terminated {
            LifecycleState::Normal
        } else {
            LifecycleState::RestartRequired
        };
    }

    pub fn begin_reserved_install_stop(
        &mut self,
        app: &AppHandle,
    ) -> Result<ReceiverStopTask, String> {
        if self.lifecycle != LifecycleState::InstallReserved {
            return Err("update installation is not reserved".to_owned());
        }
        Ok(self.begin_stop_worker(app))
    }

    pub fn mark_install_failure(&mut self, restart_required: bool) {
        self.lifecycle = if restart_required {
            LifecycleState::RestartRequired
        } else {
            LifecycleState::Normal
        };
    }

    pub fn ordinary_exit_requested(&mut self) -> bool {
        if self.lifecycle == LifecycleState::InstallReserved {
            return false;
        }
        self.lifecycle = LifecycleState::ExitAccepted;
        true
    }

    pub fn install_reserved(&self) -> bool {
        self.lifecycle == LifecycleState::InstallReserved
    }

    pub fn complete_install_handoff(&mut self) {
        if self.lifecycle == LifecycleState::InstallReserved {
            self.lifecycle = LifecycleState::ExitAccepted;
        }
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
            lifecycle: LifecycleState::Normal,
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
) -> WorkerOutcome {
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
                    virtual_controller: VirtualControllerStatus::Unknown,
                    pressed_buttons: Vec::new(),
                    packet_count: 0,
                    last_packet_at: None,
                },
            );
            return WorkerOutcome::Neutral;
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
                    virtual_controller: VirtualControllerStatus::Error(message),
                    pressed_buttons: Vec::new(),
                    packet_count: 0,
                    last_packet_at: None,
                },
            );
            return WorkerOutcome::Neutral;
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
            virtual_controller: VirtualControllerStatus::Ready,
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
                return WorkerOutcome::NeutralFailed;
            }
            return WorkerOutcome::Neutral;
        }

        match receiver.next_event() {
            Ok(ReceiverEvent::State { sender, state }) => {
                let output = map_ds_to_xbox(state);
                packet_count += 1;
                receiver_session_state.record_packet();
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
                        virtual_controller: VirtualControllerStatus::Ready,
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

                let timeout_fallback = RuntimeStatus {
                    receiver: ReceiverStatus::Running {
                        bound_address: bind_addr.to_string(),
                        last_sender: None,
                    },
                    virtual_controller: VirtualControllerStatus::Ready,
                    pressed_buttons: Vec::new(),
                    packet_count,
                    last_packet_at: None,
                };
                publish_timeout_status(&status, timeout_fallback, |timeout_status| {
                    let _ = app.emit(STATUS_EVENT, RuntimeStatusDto::from(timeout_status));
                });
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
    fn record_packet(&mut self) {
        self.timeout_status_reported = false;
    }

    fn record_update(&mut self, buttons: Buttons) {
        self.has_pressed_inputs = !buttons.is_empty();
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

fn publish_timeout_status(
    status: &Arc<Mutex<RuntimeStatus>>,
    fallback: RuntimeStatus,
    publish: impl FnOnce(RuntimeStatus),
) {
    match status.lock() {
        Ok(mut current) => {
            current.pressed_buttons.clear();
            publish(current.clone());
        }
        Err(_) => publish(fallback),
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
    fn install_reservation_rejects_exit_and_receiver_mutations() {
        let mut receiver = ReceiverController::default();

        receiver
            .reserve_install()
            .expect("install reserves lifecycle");

        assert!(!receiver.ordinary_exit_requested());
        assert!(receiver.install_reserved());
        assert!(receiver.reserve_install().is_err());
    }

    #[test]
    fn automatic_install_reservation_requires_an_already_idle_receiver() {
        let mut idle = ReceiverController::default();
        assert!(idle.reserve_install_if_idle());
        assert!(idle.install_reserved());

        let mut active = ReceiverController::default();
        let (stop_tx, _stop_rx) = mpsc::channel();
        active.stop_tx = Some(stop_tx);
        assert!(!active.reserve_install_if_idle());
        assert!(!active.install_reserved());

        let mut unsafe_state = ReceiverController::default();
        *unsafe_state.status.lock().unwrap() = RuntimeStatus {
            receiver: ReceiverStatus::Error("synthetic receiver failure".to_owned()),
            virtual_controller: VirtualControllerStatus::Error(
                "synthetic neutral-output failure".to_owned(),
            ),
            ..RuntimeStatus::default()
        };
        assert!(!unsafe_state.reserve_install_if_idle());
        assert!(!unsafe_state.install_reserved());
    }

    #[test]
    fn accepted_exit_prevents_a_later_install_reservation() {
        let mut receiver = ReceiverController::default();

        assert!(receiver.ordinary_exit_requested());

        assert!(receiver.reserve_install().is_err());
    }

    #[test]
    fn install_requires_both_worker_termination_and_neutral_output() {
        assert!(StopOutcome {
            worker_terminated: true,
            neutral: true,
        }
        .install_ready());
        assert!(!StopOutcome {
            worker_terminated: true,
            neutral: false,
        }
        .install_ready());
        assert!(!StopOutcome {
            worker_terminated: false,
            neutral: true,
        }
        .install_ready());
    }

    #[test]
    fn failed_install_preparation_reopens_only_after_worker_termination() {
        let mut terminated = ReceiverController::default();
        terminated
            .reserve_install()
            .expect("install reserves lifecycle");
        terminated.finish_install_preparation_failure(StopOutcome {
            worker_terminated: true,
            neutral: false,
        });

        let mut unknown_termination = ReceiverController::default();
        unknown_termination
            .reserve_install()
            .expect("install reserves lifecycle");
        unknown_termination.finish_install_preparation_failure(StopOutcome {
            worker_terminated: false,
            neutral: false,
        });

        assert!(terminated.reserve_install().is_ok());
        assert!(unknown_termination.reserve_install().is_err());
    }

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

        state.record_packet();

        assert!(state.needs_timeout_status());
    }

    #[test]
    fn timeout_status_preserves_last_packet_metadata() {
        use std::cell::{Cell, RefCell};

        let current = RuntimeStatus {
            receiver: ReceiverStatus::Running {
                bound_address: "0.0.0.0:26760".to_owned(),
                last_sender: Some("192.168.1.25:49152".to_owned()),
            },
            virtual_controller: VirtualControllerStatus::Ready,
            pressed_buttons: vec!["a".to_owned()],
            packet_count: 42,
            last_packet_at: Some("123456".to_owned()),
        };

        let shared = Arc::new(Mutex::new(current.clone()));
        let published = RefCell::new(None);
        let published_while_locked = Cell::new(false);
        publish_timeout_status(&shared, RuntimeStatus::default(), |status| {
            published_while_locked.set(shared.try_lock().is_err());
            published.replace(Some(status));
        });
        let timed_out = published.into_inner().expect("published timeout status");

        assert_eq!(
            timed_out,
            RuntimeStatus {
                pressed_buttons: Vec::new(),
                ..current
            }
        );
        assert!(published_while_locked.get());
        assert_eq!(*shared.lock().expect("timeout status"), timed_out);
    }
}
