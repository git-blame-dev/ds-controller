use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::{Error as UpdaterError, Update, UpdaterExt};

use crate::app_state::AppState;
use crate::dto::CommandErrorDto;
use crate::receiver_task::{ReceiverController, StopOutcome};

pub const SNAPSHOT_EVENT: &str = "updater://snapshot";
const CHECK_TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(10 * 60);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdatePhase {
    Idle,
    Current,
    Unavailable,
    Available,
    Downloading,
    Ready,
    Installing,
    RestartRequired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdateOperation {
    Check,
    Download,
    Install,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InstallMode {
    Manual,
    Automatic,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSnapshot {
    pub revision: u64,
    pub phase: UpdatePhase,
    pub operation: Option<UpdateOperation>,
    pub current_version: String,
    pub available_version: Option<String>,
    pub notes: Option<String>,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub error: Option<String>,
    pub auto_download_enabled: bool,
    pub auto_install_enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationReservation(u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateSessionError {
    Busy,
    NoAvailableUpdate,
    NoReadyUpdate,
    AutomaticCheckSuppressed,
    RetainedUpdate,
    RestartRequired,
}

enum InstallPreparation<T> {
    Proceed {
        reservation: OperationReservation,
        update: T,
        bytes: Vec<u8>,
    },
    Blocked(UpdateSnapshot),
    Deferred(UpdateSnapshot),
}

fn prepare_install<T>(
    receiver: &Mutex<ReceiverController>,
    updater: &Mutex<UpdateSession<T>>,
    mode: InstallMode,
    stop_worker: impl FnOnce() -> StopOutcome,
    mut publish: impl FnMut(UpdateSnapshot),
) -> Result<InstallPreparation<T>, CommandErrorDto> {
    let receiver_reserved = {
        let mut receiver = receiver
            .lock()
            .map_err(|_| CommandErrorDto::state_unavailable())?;
        match receiver.reserve_install() {
            Ok(()) => true,
            Err(error) if mode == InstallMode::Manual => {
                return Err(CommandErrorDto::updater_error(error));
            }
            Err(_) => false,
        }
    };
    if !receiver_reserved {
        let snapshot = updater
            .lock()
            .map_err(|_| CommandErrorDto::state_unavailable())?
            .snapshot();
        return Ok(InstallPreparation::Deferred(snapshot));
    }

    let (reservation, update, bytes) = {
        let mut session = match updater.lock() {
            Ok(session) => session,
            Err(_) => {
                if let Ok(mut receiver) = receiver.lock() {
                    receiver.cancel_install_reservation();
                }
                return Err(CommandErrorDto::state_unavailable());
            }
        };
        if mode == InstallMode::Automatic && !session.should_install_automatically() {
            let snapshot = session.snapshot();
            drop(session);
            if let Ok(mut receiver) = receiver.lock() {
                receiver.cancel_install_reservation();
            }
            return Ok(InstallPreparation::Deferred(snapshot));
        }
        let reservation = match session.reserve(UpdateOperation::Install, true) {
            Ok(reservation) => reservation,
            Err(error) => {
                drop(session);
                if let Ok(mut receiver) = receiver.lock() {
                    receiver.cancel_install_reservation();
                }
                return Err(update_command_error(error));
            }
        };
        let Some((update, bytes)) = session.take_ready(reservation) else {
            drop(session);
            if let Ok(mut receiver) = receiver.lock() {
                receiver.cancel_install_reservation();
            }
            return Err(CommandErrorDto::updater_error(
                "verified update resources are unavailable",
            ));
        };
        publish(session.snapshot());
        (reservation, update, bytes)
    };

    let outcome = stop_worker();
    if outcome.install_ready() {
        return Ok(InstallPreparation::Proceed {
            reservation,
            update,
            bytes,
        });
    }

    receiver
        .lock()
        .map_err(|_| CommandErrorDto::state_unavailable())?
        .finish_install_preparation_failure(outcome);
    let snapshot = {
        let mut session = updater
            .lock()
            .map_err(|_| CommandErrorDto::state_unavailable())?;
        let message =
            "receiver worker termination and neutral output were not both confirmed".to_owned();
        let _ = (update, bytes);
        session.finish_restart_required(reservation, message);
        session.snapshot()
    };
    publish(snapshot.clone());
    Ok(InstallPreparation::Blocked(snapshot))
}

impl fmt::Display for UpdateSessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Busy => "another update operation is already running",
            Self::NoAvailableUpdate => "there is no update available to download",
            Self::NoReadyUpdate => "there is no verified update ready to install",
            Self::AutomaticCheckSuppressed => {
                "automatic update checks are deferred for this session"
            }
            Self::RetainedUpdate => "the retained update must be downloaded or installed first",
            Self::RestartRequired => "restart is required before another update operation",
        };
        formatter.write_str(message)
    }
}

pub struct UpdateSession<T> {
    snapshot: UpdateSnapshot,
    resource: Option<(T, Option<Vec<u8>>)>,
    automatic_checks_suppressed: bool,
    next_operation_id: u64,
    active_operation_id: Option<u64>,
}

impl<T> UpdateSession<T> {
    pub fn new(
        current_version: String,
        auto_download_enabled: bool,
        auto_install_enabled: bool,
    ) -> Self {
        Self {
            snapshot: UpdateSnapshot {
                revision: 0,
                phase: UpdatePhase::Idle,
                operation: None,
                current_version,
                available_version: None,
                notes: None,
                downloaded_bytes: 0,
                total_bytes: None,
                error: None,
                auto_download_enabled,
                auto_install_enabled,
            },
            resource: None,
            automatic_checks_suppressed: false,
            next_operation_id: 0,
            active_operation_id: None,
        }
    }

    pub fn snapshot(&self) -> UpdateSnapshot {
        self.snapshot.clone()
    }

    pub fn reserve(
        &mut self,
        operation: UpdateOperation,
        manual: bool,
    ) -> Result<OperationReservation, UpdateSessionError> {
        if self.active_operation_id.is_some() {
            return Err(UpdateSessionError::Busy);
        }
        if self.snapshot.phase == UpdatePhase::RestartRequired {
            return Err(UpdateSessionError::RestartRequired);
        }
        if operation == UpdateOperation::Check
            && matches!(
                self.snapshot.phase,
                UpdatePhase::Available | UpdatePhase::Ready
            )
        {
            return Err(UpdateSessionError::RetainedUpdate);
        }
        if operation == UpdateOperation::Check && !manual && self.automatic_checks_suppressed {
            return Err(UpdateSessionError::AutomaticCheckSuppressed);
        }
        if operation == UpdateOperation::Download && !matches!(self.resource, Some((_, None))) {
            return Err(UpdateSessionError::NoAvailableUpdate);
        }
        if operation == UpdateOperation::Install && !matches!(self.resource, Some((_, Some(_)))) {
            return Err(UpdateSessionError::NoReadyUpdate);
        }

        if operation == UpdateOperation::Check && manual {
            self.automatic_checks_suppressed = false;
        }
        self.next_operation_id = self.next_operation_id.wrapping_add(1);
        self.active_operation_id = Some(self.next_operation_id);
        self.snapshot.operation = Some(operation);
        self.snapshot.error = None;
        if operation == UpdateOperation::Download {
            self.snapshot.phase = UpdatePhase::Downloading;
            self.snapshot.downloaded_bytes = 0;
            self.snapshot.total_bytes = None;
        } else if operation == UpdateOperation::Install {
            self.snapshot.phase = UpdatePhase::Installing;
        }
        self.bump_revision();
        Ok(OperationReservation(self.next_operation_id))
    }

    pub fn finish_check(
        &mut self,
        reservation: OperationReservation,
        update: Option<T>,
        available_version: Option<String>,
        notes: Option<String>,
    ) {
        if !self.finish_operation(reservation) {
            return;
        }
        self.resource = update.map(|update| (update, None));
        self.snapshot.available_version = available_version;
        self.snapshot.notes = notes;
        self.snapshot.downloaded_bytes = 0;
        self.snapshot.total_bytes = None;
        self.snapshot.phase = if self.resource.is_some() {
            UpdatePhase::Available
        } else {
            UpdatePhase::Current
        };
    }

    pub fn update_download_progress(
        &mut self,
        reservation: OperationReservation,
        chunk_length: usize,
        total: Option<u64>,
    ) -> bool {
        if self.active_operation_id != Some(reservation.0)
            || self.snapshot.operation != Some(UpdateOperation::Download)
        {
            return false;
        }
        self.snapshot.downloaded_bytes = self
            .snapshot
            .downloaded_bytes
            .saturating_add(chunk_length as u64);
        self.snapshot.total_bytes = total;
        self.bump_revision();
        true
    }

    pub fn finish_download(&mut self, reservation: OperationReservation, bytes: Vec<u8>) -> bool {
        if !self.finish_operation(reservation) {
            return false;
        }
        if let Some((update, _)) = self.resource.take() {
            self.resource = Some((update, Some(bytes)));
            self.snapshot.phase = UpdatePhase::Ready;
        }
        self.should_install_automatically()
    }

    pub fn finish_failure(&mut self, reservation: OperationReservation, message: String) {
        if !self.finish_operation(reservation) {
            return;
        }
        self.snapshot.phase = match self.snapshot.phase {
            UpdatePhase::Downloading if self.resource.is_some() => UpdatePhase::Available,
            UpdatePhase::Installing if self.resource.is_some() => UpdatePhase::Ready,
            phase => phase,
        };
        self.snapshot.error = Some(message);
    }

    pub fn finish_unavailable(&mut self, reservation: OperationReservation) {
        if !self.finish_operation(reservation) {
            return;
        }
        self.resource = None;
        self.snapshot.phase = UpdatePhase::Unavailable;
        self.snapshot.available_version = None;
        self.snapshot.notes = None;
        self.snapshot.downloaded_bytes = 0;
        self.snapshot.total_bytes = None;
        self.snapshot.error = None;
    }

    pub fn take_ready(&mut self, reservation: OperationReservation) -> Option<(T, Vec<u8>)> {
        if self.active_operation_id != Some(reservation.0) {
            return None;
        }
        self.resource
            .take()
            .and_then(|(update, bytes)| bytes.map(|bytes| (update, bytes)))
    }

    pub fn restore_ready(
        &mut self,
        reservation: OperationReservation,
        update: T,
        bytes: Vec<u8>,
        message: String,
    ) {
        self.resource = Some((update, Some(bytes)));
        self.finish_failure(reservation, message);
    }

    pub fn finish_restart_required(&mut self, reservation: OperationReservation, message: String) {
        if self.finish_operation(reservation) {
            self.resource = None;
            self.snapshot.phase = UpdatePhase::RestartRequired;
            self.snapshot.error = Some(message);
        }
    }

    fn finish_lost_install_operation(&mut self, message: String) {
        self.active_operation_id = None;
        self.snapshot.operation = None;
        self.resource = None;
        self.snapshot.phase = UpdatePhase::RestartRequired;
        self.snapshot.error = Some(message);
        self.bump_revision();
    }

    pub fn defer(&mut self) -> Result<(), UpdateSessionError> {
        if self.active_operation_id.is_some() {
            return Err(UpdateSessionError::Busy);
        }
        if !matches!(self.resource, Some((_, Some(_)))) {
            return Err(UpdateSessionError::NoReadyUpdate);
        }
        self.resource = None;
        self.snapshot.phase = UpdatePhase::Idle;
        self.snapshot.available_version = None;
        self.snapshot.notes = None;
        self.snapshot.downloaded_bytes = 0;
        self.snapshot.total_bytes = None;
        self.snapshot.error = None;
        self.automatic_checks_suppressed = true;
        self.bump_revision();
        Ok(())
    }

    pub fn set_auto_download_enabled(&mut self, enabled: bool) {
        self.snapshot.auto_download_enabled = enabled;
        self.bump_revision();
    }

    pub fn set_auto_install_enabled(&mut self, enabled: bool) -> bool {
        self.snapshot.auto_install_enabled = enabled;
        self.bump_revision();
        self.should_install_automatically()
    }

    fn should_install_automatically(&self) -> bool {
        self.snapshot.auto_install_enabled
            && self.snapshot.phase == UpdatePhase::Ready
            && self.snapshot.operation.is_none()
            && matches!(self.resource, Some((_, Some(_))))
    }

    fn finish_operation(&mut self, reservation: OperationReservation) -> bool {
        if self.active_operation_id != Some(reservation.0) {
            return false;
        }
        self.active_operation_id = None;
        self.snapshot.operation = None;
        self.bump_revision();
        true
    }

    fn bump_revision(&mut self) {
        self.snapshot.revision = self.snapshot.revision.saturating_add(1);
    }

    #[cfg(test)]
    fn phase(&self) -> UpdatePhase {
        self.snapshot.phase
    }

    #[cfg(test)]
    fn has_payload(&self) -> bool {
        matches!(self.resource, Some((_, Some(_))))
    }

    #[cfg(test)]
    fn automatic_checks_suppressed(&self) -> bool {
        self.automatic_checks_suppressed
    }
}

pub fn start_background_check(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let _ = perform_check(app, false).await;
    });
}

fn finish_lost_install_task<T>(
    receiver: &Mutex<ReceiverController>,
    updater: &Mutex<UpdateSession<T>>,
    message: String,
) -> UpdateSnapshot {
    let mut receiver_guard = match receiver.lock() {
        Ok(receiver) => receiver,
        Err(poisoned) => poisoned.into_inner(),
    };
    receiver_guard.finish_install_preparation_failure(StopOutcome {
        worker_terminated: false,
        neutral: false,
    });
    receiver.clear_poison();
    drop(receiver_guard);

    let mut session = match updater.lock() {
        Ok(session) => session,
        Err(poisoned) => poisoned.into_inner(),
    };
    session.finish_lost_install_operation(message);
    let snapshot = session.snapshot();
    updater.clear_poison();
    drop(session);
    snapshot
}

fn finish_returned_install_error<T>(
    receiver: &Mutex<ReceiverController>,
    updater: &Mutex<UpdateSession<T>>,
    reservation: OperationReservation,
    update: T,
    bytes: Vec<u8>,
    message: String,
) -> Result<UpdateSnapshot, CommandErrorDto> {
    if receiver.is_poisoned() || updater.is_poisoned() {
        return Ok(finish_lost_install_task(receiver, updater, message));
    }

    let mut receiver_guard = match receiver.lock() {
        Ok(receiver) => receiver,
        Err(poisoned) => {
            drop(poisoned.into_inner());
            return Ok(finish_lost_install_task(receiver, updater, message));
        }
    };
    #[cfg(target_os = "linux")]
    receiver_guard.mark_install_failure(true);
    #[cfg(windows)]
    receiver_guard.mark_install_failure(false);
    drop(receiver_guard);

    let mut session = match updater.lock() {
        Ok(session) => session,
        Err(poisoned) => {
            drop(poisoned.into_inner());
            return Ok(finish_lost_install_task(receiver, updater, message));
        }
    };
    #[cfg(target_os = "linux")]
    {
        let _ = (update, bytes);
        session.finish_restart_required(reservation, message);
    }
    #[cfg(windows)]
    session.restore_ready(reservation, update, bytes, message);
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = (reservation, update, bytes, message);
        return Err(CommandErrorDto::updater_error(
            "unsupported update platform",
        ));
    }
    Ok(session.snapshot())
}

#[tauri::command]
pub fn get_update_snapshot(state: State<'_, AppState>) -> Result<UpdateSnapshot, CommandErrorDto> {
    snapshot_from_state(state.inner())
}

#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> Result<UpdateSnapshot, CommandErrorDto> {
    perform_check(app, true).await
}

#[tauri::command]
pub async fn download_update(app: AppHandle) -> Result<UpdateSnapshot, CommandErrorDto> {
    perform_download(app).await
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<UpdateSnapshot, CommandErrorDto> {
    perform_install(app, InstallMode::Manual).await
}

async fn perform_install(
    app: AppHandle,
    mode: InstallMode,
) -> Result<UpdateSnapshot, CommandErrorDto> {
    ensure_bundle_eligible()?;
    let receiver = app.state::<AppState>().receiver().clone();
    let updater_state = app.state::<AppState>().updater().clone();
    let preparation_receiver = receiver.clone();
    let preparation_updater = updater_state.clone();
    let stop_receiver = receiver.clone();
    let stop_app = app.clone();
    let publish_app = app.clone();
    let preparation_task = tauri::async_runtime::spawn_blocking(move || {
        prepare_install(
            &preparation_receiver,
            &preparation_updater,
            mode,
            || {
                let stop_task = stop_receiver
                    .lock()
                    .ok()
                    .and_then(|mut receiver| receiver.begin_reserved_install_stop(&stop_app).ok());
                let Some(stop_task) = stop_task else {
                    return StopOutcome {
                        worker_terminated: false,
                        neutral: false,
                    };
                };
                let outcome = stop_task.join();
                if let Ok(mut receiver) = stop_receiver.lock() {
                    receiver.finish_stop(&stop_app, outcome);
                }
                outcome
            },
            |snapshot| {
                let _ = publish_app.emit(SNAPSHOT_EVENT, snapshot);
            },
        )
    })
    .await;
    let preparation = match preparation_task {
        Ok(preparation) => preparation,
        Err(error) => {
            let snapshot = finish_lost_install_task(
                &receiver,
                &updater_state,
                format!(
                    "update preparation task was lost: {error}. Restart DS Controller before using the receiver."
                ),
            );
            let _ = app.emit(SNAPSHOT_EVENT, snapshot.clone());
            return Ok(snapshot);
        }
    };
    let (reservation, update, bytes) = match preparation? {
        InstallPreparation::Proceed {
            reservation,
            update,
            bytes,
        } => (reservation, update, bytes),
        InstallPreparation::Blocked(snapshot) => return Ok(snapshot),
        InstallPreparation::Deferred(snapshot) => return Ok(snapshot),
    };

    let install_task = tauri::async_runtime::spawn_blocking(move || {
        let result = update.install(&bytes).map_err(|error| error.to_string());
        (result, update, bytes)
    })
    .await;
    let install_result = match install_task {
        Ok(result) => result,
        Err(error) => {
            let snapshot = finish_lost_install_task(
                &receiver,
                &updater_state,
                format!("installer task was lost: {error}"),
            );
            let _ = app.emit(SNAPSHOT_EVENT, snapshot.clone());
            return Ok(snapshot);
        }
    };

    match install_result {
        (Ok(()), _, _) => {
            #[cfg(target_os = "linux")]
            {
                if let Ok(mut receiver) = receiver.lock() {
                    receiver.complete_install_handoff();
                }
                app.restart();
            }
            #[allow(unreachable_code)]
            snapshot_from_arc(&updater_state)
        }
        (Err(message), update, bytes) => {
            #[cfg(target_os = "linux")]
            let message = format!(
                "{message}. Restart DS Controller and check the Debian package state before using the receiver."
            );
            #[cfg(windows)]
            let message = message;
            let snapshot = finish_returned_install_error(
                &receiver,
                &updater_state,
                reservation,
                update,
                bytes,
                message,
            )?;
            let _ = app.emit(SNAPSHOT_EVENT, snapshot.clone());
            Ok(snapshot)
        }
    }
}

#[tauri::command]
pub fn defer_update(app: AppHandle) -> Result<UpdateSnapshot, CommandErrorDto> {
    let updater = app.state::<AppState>().updater().clone();
    let snapshot = {
        let mut session = updater
            .lock()
            .map_err(|_| CommandErrorDto::state_unavailable())?;
        session.defer().map_err(update_command_error)?;
        session.snapshot()
    };
    let _ = app.emit(SNAPSHOT_EVENT, snapshot.clone());
    Ok(snapshot)
}

#[tauri::command]
pub fn set_auto_download_updates(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<UpdateSnapshot, CommandErrorDto> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|error| CommandErrorDto::invalid_settings(error.to_string()))?;
    {
        let mut settings = state
            .settings()
            .lock()
            .map_err(|_| CommandErrorDto::state_unavailable())?;
        let mut next = settings.clone();
        next.auto_download_updates = enabled;
        crate::settings::save_settings(&config_dir, &next)
            .map_err(|error| CommandErrorDto::invalid_settings(error.to_string()))?;
        *settings = next;
    }
    let snapshot = {
        let mut session = state
            .updater()
            .lock()
            .map_err(|_| CommandErrorDto::state_unavailable())?;
        session.set_auto_download_enabled(enabled);
        session.snapshot()
    };
    let _ = app.emit(SNAPSHOT_EVENT, snapshot.clone());
    Ok(snapshot)
}

enum AutoInstallPreferenceUpdate {
    Saved {
        snapshot: UpdateSnapshot,
        should_install: bool,
    },
    Reverted {
        error: CommandErrorDto,
        snapshot: UpdateSnapshot,
        should_install: bool,
    },
}

fn set_auto_install_preference<T>(
    operation: &Mutex<()>,
    settings: &Mutex<crate::settings::AppSettings>,
    updater: &Mutex<UpdateSession<T>>,
    enabled: bool,
    persist: impl FnOnce(&crate::settings::AppSettings) -> Result<(), CommandErrorDto>,
) -> Result<AutoInstallPreferenceUpdate, CommandErrorDto> {
    let _operation = operation
        .lock()
        .map_err(|_| CommandErrorDto::state_unavailable())?;

    if enabled {
        let mut settings = settings
            .lock()
            .map_err(|_| CommandErrorDto::state_unavailable())?;
        let mut next = settings.clone();
        next.auto_install_updates = true;
        persist(&next)?;
        *settings = next;
        drop(settings);

        let mut session = updater
            .lock()
            .map_err(|_| CommandErrorDto::state_unavailable())?;
        let should_install = session.set_auto_install_enabled(true);
        return Ok(AutoInstallPreferenceUpdate::Saved {
            snapshot: session.snapshot(),
            should_install,
        });
    }

    let previous = {
        let mut session = updater
            .lock()
            .map_err(|_| CommandErrorDto::state_unavailable())?;
        let previous = session.snapshot().auto_install_enabled;
        session.set_auto_install_enabled(false);
        previous
    };
    let persisted = (|| {
        let mut settings = settings
            .lock()
            .map_err(|_| CommandErrorDto::state_unavailable())?;
        let mut next = settings.clone();
        next.auto_install_updates = false;
        persist(&next)?;
        *settings = next;
        Ok(())
    })();
    if let Err(error) = persisted {
        let mut session = updater
            .lock()
            .map_err(|_| CommandErrorDto::state_unavailable())?;
        let should_install = session.set_auto_install_enabled(previous);
        return Ok(AutoInstallPreferenceUpdate::Reverted {
            error,
            snapshot: session.snapshot(),
            should_install,
        });
    }

    let session = updater
        .lock()
        .map_err(|_| CommandErrorDto::state_unavailable())?;
    Ok(AutoInstallPreferenceUpdate::Saved {
        snapshot: session.snapshot(),
        should_install: false,
    })
}

#[tauri::command]
pub async fn set_auto_install_updates(
    app: AppHandle,
    enabled: bool,
) -> Result<UpdateSnapshot, CommandErrorDto> {
    let state = app.state::<AppState>();
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|error| CommandErrorDto::invalid_settings(error.to_string()))?;
    let outcome = set_auto_install_preference(
        state.auto_install_preference(),
        state.settings(),
        state.updater(),
        enabled,
        |next| {
            crate::settings::save_settings(&config_dir, next)
                .map_err(|error| CommandErrorDto::invalid_settings(error.to_string()))
        },
    )?;
    let (snapshot, should_install, persistence_error) = match outcome {
        AutoInstallPreferenceUpdate::Saved {
            snapshot,
            should_install,
        } => (snapshot, should_install, None),
        AutoInstallPreferenceUpdate::Reverted {
            error,
            snapshot,
            should_install,
        } => (snapshot, should_install, Some(error)),
    };
    let _ = app.emit(SNAPSHOT_EVENT, snapshot.clone());
    if should_install {
        let install_result = perform_install(app, InstallMode::Automatic).await;
        if let Some(error) = persistence_error {
            let _ = install_result;
            return Err(error);
        }
        return install_result;
    }
    persistence_error.map_or(Ok(snapshot), Err)
}

async fn perform_check(app: AppHandle, manual: bool) -> Result<UpdateSnapshot, CommandErrorDto> {
    ensure_bundle_eligible()?;
    let updater_state = app.state::<AppState>().updater().clone();
    let reservation = {
        let mut session = updater_state
            .lock()
            .map_err(|_| CommandErrorDto::state_unavailable())?;
        let reservation = session
            .reserve(UpdateOperation::Check, manual)
            .map_err(update_command_error)?;
        let snapshot = session.snapshot();
        let _ = app.emit(SNAPSHOT_EVENT, snapshot);
        reservation
    };

    let checked = match updater_builder(&app).build() {
        Ok(updater) => updater.check().await,
        Err(error) => Err(error),
    };

    let should_download = {
        let mut session = updater_state
            .lock()
            .map_err(|_| CommandErrorDto::state_unavailable())?;
        match checked {
            Ok(update) => {
                let version = update.as_ref().map(|update| update.version.clone());
                let notes = update.as_ref().and_then(|update| update.body.clone());
                session.finish_check(reservation, update, version, notes);
            }
            Err(UpdaterError::ReleaseNotFound) => session.finish_unavailable(reservation),
            Err(error) => session.finish_failure(reservation, error.to_string()),
        }
        let snapshot = session.snapshot();
        let _ = app.emit(SNAPSHOT_EVENT, snapshot.clone());
        snapshot.phase == UpdatePhase::Available && snapshot.auto_download_enabled && !manual
    };

    if should_download {
        perform_download(app).await
    } else {
        snapshot_from_arc(&updater_state)
    }
}

async fn perform_download(app: AppHandle) -> Result<UpdateSnapshot, CommandErrorDto> {
    let updater_state = app.state::<AppState>().updater().clone();
    let (reservation, mut update) = {
        let mut session = updater_state
            .lock()
            .map_err(|_| CommandErrorDto::state_unavailable())?;
        let reservation = session
            .reserve(UpdateOperation::Download, true)
            .map_err(update_command_error)?;
        let update = session
            .resource
            .as_ref()
            .map(|(update, _)| update.clone())
            .ok_or_else(|| update_command_error(UpdateSessionError::NoAvailableUpdate))?;
        let _ = app.emit(SNAPSHOT_EVENT, session.snapshot());
        (reservation, update)
    };
    update.timeout = Some(DOWNLOAD_TIMEOUT);

    let progress_state = Arc::clone(&updater_state);
    let progress_app = app.clone();
    let downloaded = update
        .download(
            move |chunk_length, total| {
                if let Ok(mut session) = progress_state.lock() {
                    if session.update_download_progress(reservation, chunk_length, total) {
                        let _ = progress_app.emit(SNAPSHOT_EVENT, session.snapshot());
                    }
                }
            },
            || {},
        )
        .await;

    let (snapshot, should_install) = {
        let mut session = updater_state
            .lock()
            .map_err(|_| CommandErrorDto::state_unavailable())?;
        let should_install = match downloaded {
            Ok(bytes) => session.finish_download(reservation, bytes),
            Err(error) => {
                session.finish_failure(reservation, error.to_string());
                false
            }
        };
        (session.snapshot(), should_install)
    };
    let _ = app.emit(SNAPSHOT_EVENT, snapshot.clone());
    if should_install {
        perform_install(app, InstallMode::Automatic).await
    } else {
        Ok(snapshot)
    }
}

fn updater_builder(app: &AppHandle) -> tauri_plugin_updater::UpdaterBuilder {
    let builder = app.updater_builder().timeout(CHECK_TIMEOUT);
    #[cfg(windows)]
    let builder = builder.on_before_exit(|| {});
    builder
}

fn snapshot_from_state(state: &AppState) -> Result<UpdateSnapshot, CommandErrorDto> {
    snapshot_from_arc(state.updater())
}

fn snapshot_from_arc(
    state: &Arc<Mutex<UpdateSession<Update>>>,
) -> Result<UpdateSnapshot, CommandErrorDto> {
    state
        .lock()
        .map(|session| session.snapshot())
        .map_err(|_| CommandErrorDto::state_unavailable())
}

fn update_command_error(error: UpdateSessionError) -> CommandErrorDto {
    CommandErrorDto::updater_error(error.to_string())
}

fn ensure_bundle_eligible() -> Result<(), CommandErrorDto> {
    use tauri::utils::config::BundleType;
    use tauri::utils::platform::bundle_type;

    match bundle_type() {
        #[cfg(windows)]
        Some(BundleType::Nsis) => Ok(()),
        #[cfg(target_os = "linux")]
        Some(BundleType::Deb)
            if std::env::current_exe()
                .map(|path| path == std::path::Path::new("/usr/bin/ds-controller"))
                .unwrap_or(false) =>
        {
            Ok(())
        }
        _ => Err(CommandErrorDto::updater_error(
            "updates are available only from an installed DS Controller NSIS or Debian package",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stop_outcome(worker_terminated: bool, neutral: bool) -> StopOutcome {
        StopOutcome {
            worker_terminated,
            neutral,
        }
    }

    fn ready_session() -> Mutex<UpdateSession<()>> {
        let mut session = UpdateSession::new("1.0.0".to_owned(), true, true);
        session.resource = Some(((), Some(vec![1, 2, 3])));
        session.snapshot.phase = UpdatePhase::Ready;
        Mutex::new(session)
    }

    #[test]
    fn missing_release_metadata_is_unavailable_instead_of_an_update_error() {
        let mut session = UpdateSession::<()>::new("1.0.0".to_owned(), true, true);
        let reservation = session
            .reserve(UpdateOperation::Check, false)
            .expect("check reserves the session");

        session.finish_unavailable(reservation);

        assert_eq!(session.phase(), UpdatePhase::Unavailable);
        assert_eq!(session.snapshot().error, None);
    }

    #[test]
    fn automatic_startup_check_is_allowed_when_auto_download_is_disabled() {
        let mut session = UpdateSession::<()>::new("1.0.0".to_owned(), false, true);

        let reservation = session
            .reserve(UpdateOperation::Check, false)
            .expect("startup metadata check reserves independently of download preference");

        assert_eq!(session.snapshot().operation, Some(UpdateOperation::Check));
        session.finish_unavailable(reservation);
        assert!(!session.snapshot().auto_download_enabled);
    }

    #[test]
    fn install_preparation_proceeds_after_reserving_and_stopping_neutral_worker() {
        let receiver = Mutex::new(crate::receiver_task::ReceiverController::default());
        let updater = ready_session();

        let preparation = prepare_install(
            &receiver,
            &updater,
            InstallMode::Manual,
            || {
                assert!(receiver.lock().unwrap().install_reserved());
                stop_outcome(true, true)
            },
            |_| {},
        )
        .expect("installation preparation succeeds");

        assert!(matches!(preparation, InstallPreparation::Proceed { .. }));
        assert!(receiver.lock().unwrap().install_reserved());
        assert_eq!(updater.lock().unwrap().phase(), UpdatePhase::Installing);
    }

    #[test]
    fn install_preparation_requires_restart_after_neutral_failure() {
        let receiver = Mutex::new(crate::receiver_task::ReceiverController::default());
        let updater = ready_session();

        let preparation = prepare_install(
            &receiver,
            &updater,
            InstallMode::Manual,
            || stop_outcome(true, false),
            |_| {},
        )
        .expect("known termination is handled");

        assert!(matches!(preparation, InstallPreparation::Blocked(_)));
        assert!(receiver.lock().unwrap().reserve_install().is_err());
        let session = updater.lock().unwrap();
        assert_eq!(session.phase(), UpdatePhase::RestartRequired);
        assert!(!session.has_payload());
    }

    #[test]
    fn install_preparation_requires_restart_after_unknown_worker_termination() {
        let receiver = Mutex::new(crate::receiver_task::ReceiverController::default());
        let updater = ready_session();

        let preparation = prepare_install(
            &receiver,
            &updater,
            InstallMode::Manual,
            || stop_outcome(false, false),
            |_| {},
        )
        .expect("unknown termination is handled");

        assert!(matches!(preparation, InstallPreparation::Blocked(_)));
        assert!(receiver.lock().unwrap().reserve_install().is_err());
        let session = updater.lock().unwrap();
        assert_eq!(session.phase(), UpdatePhase::RestartRequired);
        assert!(!session.has_payload());
    }

    #[test]
    fn panicked_preparation_task_requires_restart_and_allows_close() {
        let receiver = Arc::new(Mutex::new(
            crate::receiver_task::ReceiverController::default(),
        ));
        let updater = Arc::new(ready_session());
        let task_receiver = Arc::clone(&receiver);
        let task_updater = Arc::clone(&updater);

        let task = tauri::async_runtime::block_on(async move {
            tauri::async_runtime::spawn_blocking(move || {
                prepare_install(
                    &task_receiver,
                    &task_updater,
                    InstallMode::Manual,
                    || panic!("synthetic stop callback panic"),
                    |_| {},
                )
            })
            .await
        });
        assert!(task.is_err());

        let snapshot = finish_lost_install_task(
            &receiver,
            &updater,
            "synthetic preparation task loss".to_owned(),
        );

        assert_eq!(snapshot.phase, UpdatePhase::RestartRequired);
        assert_eq!(snapshot.operation, None);
        assert!(!updater.lock().unwrap().has_payload());
        let mut receiver = receiver.lock().unwrap();
        assert!(receiver.ordinary_exit_requested());
        assert!(receiver.reserve_install().is_err());
    }

    #[test]
    fn preparation_panic_while_receiver_locked_recovers_close_access() {
        let receiver = Arc::new(Mutex::new(
            crate::receiver_task::ReceiverController::default(),
        ));
        let updater = Arc::new(ready_session());
        let task_receiver = Arc::clone(&receiver);
        let task_updater = Arc::clone(&updater);
        let panic_receiver = Arc::clone(&receiver);

        let task = tauri::async_runtime::block_on(async move {
            tauri::async_runtime::spawn_blocking(move || {
                prepare_install(
                    &task_receiver,
                    &task_updater,
                    InstallMode::Manual,
                    || {
                        let _receiver = panic_receiver.lock().unwrap();
                        panic!("synthetic panic while receiver is locked");
                    },
                    |_| {},
                )
            })
            .await
        });
        assert!(task.is_err());
        assert!(receiver.is_poisoned());

        let snapshot = finish_lost_install_task(
            &receiver,
            &updater,
            "synthetic preparation task loss".to_owned(),
        );

        assert_eq!(snapshot.phase, UpdatePhase::RestartRequired);
        assert!(!receiver.is_poisoned());
        assert!(receiver.lock().unwrap().ordinary_exit_requested());
    }

    #[test]
    fn preparation_panic_while_updater_locked_recovers_restart_state() {
        let receiver = Arc::new(Mutex::new(
            crate::receiver_task::ReceiverController::default(),
        ));
        let updater = Arc::new(ready_session());
        let task_receiver = Arc::clone(&receiver);
        let task_updater = Arc::clone(&updater);

        let task = tauri::async_runtime::block_on(async move {
            tauri::async_runtime::spawn_blocking(move || {
                prepare_install(
                    &task_receiver,
                    &task_updater,
                    InstallMode::Manual,
                    || stop_outcome(true, true),
                    |_| panic!("synthetic publish panic while updater is locked"),
                )
            })
            .await
        });
        assert!(task.is_err());
        assert!(updater.is_poisoned());

        let snapshot = finish_lost_install_task(
            &receiver,
            &updater,
            "synthetic preparation task loss".to_owned(),
        );

        assert_eq!(snapshot.phase, UpdatePhase::RestartRequired);
        assert!(!updater.is_poisoned());
        assert_eq!(updater.lock().unwrap().snapshot(), snapshot);
        assert!(receiver.lock().unwrap().ordinary_exit_requested());
    }

    #[test]
    fn panicked_installer_task_requires_restart_on_every_platform() {
        let receiver = Arc::new(Mutex::new(
            crate::receiver_task::ReceiverController::default(),
        ));
        let updater = Arc::new(ready_session());
        receiver.lock().unwrap().reserve_install().unwrap();
        let reservation = updater
            .lock()
            .unwrap()
            .reserve(UpdateOperation::Install, true)
            .unwrap();
        updater.lock().unwrap().take_ready(reservation).unwrap();

        let task = tauri::async_runtime::block_on(async {
            tauri::async_runtime::spawn_blocking(|| panic!("synthetic installer panic")).await
        });
        assert!(task.is_err());

        let snapshot = finish_lost_install_task(
            &receiver,
            &updater,
            "synthetic installer task loss".to_owned(),
        );

        assert_eq!(snapshot.phase, UpdatePhase::RestartRequired);
        assert_eq!(snapshot.operation, None);
        assert!(receiver.lock().unwrap().ordinary_exit_requested());
    }

    #[test]
    fn returned_installer_error_with_poisoned_receiver_discards_payload_and_allows_close() {
        let receiver = Arc::new(Mutex::new(
            crate::receiver_task::ReceiverController::default(),
        ));
        let updater = ready_session();
        receiver.lock().unwrap().reserve_install().unwrap();
        let reservation = updater
            .lock()
            .unwrap()
            .reserve(UpdateOperation::Install, true)
            .unwrap();
        let (update, bytes) = updater.lock().unwrap().take_ready(reservation).unwrap();
        let poison_receiver = Arc::clone(&receiver);
        assert!(std::thread::spawn(move || {
            let _receiver = poison_receiver.lock().unwrap();
            panic!("synthetic receiver state panic");
        })
        .join()
        .is_err());

        let snapshot = finish_returned_install_error(
            &receiver,
            &updater,
            reservation,
            update,
            bytes,
            "synthetic returned installer error".to_owned(),
        )
        .expect("poisoned error completion becomes conservative recovery");

        assert_eq!(snapshot.phase, UpdatePhase::RestartRequired);
        assert_eq!(snapshot.operation, None);
        assert!(!updater.lock().unwrap().has_payload());
        assert!(!receiver.is_poisoned());
        assert!(receiver.lock().unwrap().ordinary_exit_requested());
    }

    #[test]
    fn later_discards_ready_payload_and_suppresses_automatic_rediscovery() {
        let mut session = UpdateSession::new("1.0.0".to_owned(), true, true);
        session.resource = Some(((), Some(vec![1, 2, 3])));
        session.snapshot.phase = UpdatePhase::Ready;

        session.defer().expect("ready update can be deferred");

        assert_eq!(session.phase(), UpdatePhase::Idle);
        assert!(!session.has_payload());
        assert!(session.automatic_checks_suppressed());
    }

    #[test]
    fn successful_check_without_an_update_marks_the_version_current() {
        let mut session = UpdateSession::<()>::new("1.0.0".to_owned(), true, true);
        let reservation = session
            .reserve(UpdateOperation::Check, true)
            .expect("check reserves the session");

        session.finish_check(reservation, None, None, None);

        assert_eq!(session.phase(), UpdatePhase::Current);
    }

    #[test]
    fn busy_operation_rejects_another_operation_without_queueing() {
        let mut session = UpdateSession::<()>::new("1.0.0".to_owned(), true, true);
        let reservation = session
            .reserve(UpdateOperation::Check, false)
            .expect("first operation reserves");

        assert_eq!(
            session.reserve(UpdateOperation::Download, true),
            Err(UpdateSessionError::Busy)
        );

        session.finish_failure(reservation, "offline".to_owned());
        assert_eq!(session.snapshot().error.as_deref(), Some("offline"));
    }

    #[test]
    fn checks_do_not_replace_retained_available_or_ready_updates() {
        let mut available = UpdateSession::new("1.0.0".to_owned(), false, true);
        available.resource = Some(((), None));
        available.snapshot.phase = UpdatePhase::Available;
        let mut ready = UpdateSession::new("1.0.0".to_owned(), false, true);
        ready.resource = Some(((), Some(vec![1])));
        ready.snapshot.phase = UpdatePhase::Ready;

        assert!(available.reserve(UpdateOperation::Check, true).is_err());
        assert!(ready.reserve(UpdateOperation::Check, false).is_err());
        assert_eq!(available.phase(), UpdatePhase::Available);
        assert_eq!(ready.phase(), UpdatePhase::Ready);
        assert!(ready.has_payload());
    }

    #[test]
    fn restart_required_rejects_every_update_operation() {
        for operation in [
            UpdateOperation::Check,
            UpdateOperation::Download,
            UpdateOperation::Install,
        ] {
            let mut session = UpdateSession::new("1.0.0".to_owned(), true, true);
            session.snapshot.phase = UpdatePhase::RestartRequired;
            session.resource = Some(((), Some(vec![1])));

            assert!(session.reserve(operation, true).is_err());
            assert_eq!(session.phase(), UpdatePhase::RestartRequired);
        }
    }

    #[test]
    fn automatic_download_preference_changes_preserve_ready_recovery() {
        let mut session = ready_session().into_inner().unwrap();

        session.set_auto_download_enabled(false);

        assert_eq!(session.phase(), UpdatePhase::Ready);
        assert!(session.has_payload());
        assert!(!session.snapshot().auto_download_enabled);
    }

    #[test]
    fn automatic_install_preference_changes_preserve_ready_recovery() {
        let mut session = ready_session().into_inner().unwrap();

        let should_install = session.set_auto_install_enabled(false);

        assert!(!should_install);
        assert_eq!(session.phase(), UpdatePhase::Ready);
        assert!(session.has_payload());
        assert!(!session.snapshot().auto_install_enabled);
    }

    #[test]
    fn disabling_automatic_install_takes_effect_before_persistence() {
        let operation = Mutex::new(());
        let settings = Mutex::new(crate::settings::AppSettings::default());
        let updater = ready_session();

        let outcome = set_auto_install_preference(&operation, &settings, &updater, false, |_| {
            assert!(!updater.lock().unwrap().snapshot().auto_install_enabled);
            Ok(())
        })
        .expect("preference disable persists");
        let AutoInstallPreferenceUpdate::Saved {
            snapshot,
            should_install,
        } = outcome
        else {
            panic!("successful persistence must return a saved outcome");
        };

        assert!(!should_install);
        assert!(!snapshot.auto_install_enabled);
        assert!(!settings.lock().unwrap().auto_install_updates);
        assert!(updater.lock().unwrap().has_payload());
    }

    #[test]
    fn failed_automatic_install_disable_restores_the_prior_runtime_value() {
        let operation = Mutex::new(());
        let settings = Mutex::new(crate::settings::AppSettings::default());
        let updater = ready_session();

        let outcome = set_auto_install_preference(&operation, &settings, &updater, false, |_| {
            Err(CommandErrorDto::invalid_settings("synthetic save failure"))
        })
        .expect("save failure returns a publishable rollback");

        assert!(matches!(
            outcome,
            AutoInstallPreferenceUpdate::Reverted {
                should_install: true,
                ..
            }
        ));
        assert!(settings.lock().unwrap().auto_install_updates);
        assert!(updater.lock().unwrap().snapshot().auto_install_enabled);
        assert!(updater.lock().unwrap().has_payload());
    }

    #[test]
    fn failed_disable_requests_install_when_download_becomes_ready_during_persistence() {
        let operation = Mutex::new(());
        let settings = Mutex::new(crate::settings::AppSettings::default());
        let mut session = UpdateSession::new("1.0.0".to_owned(), true, true);
        session.resource = Some(((), None));
        session.snapshot.phase = UpdatePhase::Available;
        let reservation = session
            .reserve(UpdateOperation::Download, true)
            .expect("synthetic update can download");
        let updater = Mutex::new(session);

        let outcome = set_auto_install_preference(&operation, &settings, &updater, false, |_| {
            assert!(!updater
                .lock()
                .unwrap()
                .finish_download(reservation, vec![1, 2, 3]));
            Err(CommandErrorDto::invalid_settings("synthetic save failure"))
        })
        .expect("save failure returns a publishable rollback");

        let AutoInstallPreferenceUpdate::Reverted {
            snapshot,
            should_install,
            ..
        } = outcome
        else {
            panic!("failed persistence must return a rollback outcome");
        };
        assert!(should_install);
        assert!(snapshot.auto_install_enabled);
        assert_eq!(snapshot.phase, UpdatePhase::Ready);
        assert!(updater.lock().unwrap().has_payload());
    }

    #[test]
    fn overlapping_automatic_install_toggles_are_serialized() {
        use std::sync::mpsc;

        let operation = Arc::new(Mutex::new(()));
        let settings = Arc::new(Mutex::new(crate::settings::AppSettings::default()));
        let updater = Arc::new(ready_session());
        let (disable_entered_tx, disable_entered_rx) = mpsc::channel();
        let (release_disable_tx, release_disable_rx) = mpsc::channel();
        let (enable_started_tx, enable_started_rx) = mpsc::channel();
        let (enable_entered_tx, enable_entered_rx) = mpsc::channel();

        let disable_operation = Arc::clone(&operation);
        let disable_settings = Arc::clone(&settings);
        let disable_updater = Arc::clone(&updater);
        let disable = std::thread::spawn(move || {
            set_auto_install_preference(
                &disable_operation,
                &disable_settings,
                &disable_updater,
                false,
                |_| {
                    disable_entered_tx.send(()).unwrap();
                    release_disable_rx.recv().unwrap();
                    Ok(())
                },
            )
        });
        disable_entered_rx.recv().unwrap();

        let enable_operation = Arc::clone(&operation);
        let enable_settings = Arc::clone(&settings);
        let enable_updater = Arc::clone(&updater);
        let enable = std::thread::spawn(move || {
            enable_started_tx.send(()).unwrap();
            set_auto_install_preference(
                &enable_operation,
                &enable_settings,
                &enable_updater,
                true,
                |_| {
                    enable_entered_tx.send(()).unwrap();
                    Ok(())
                },
            )
        });

        enable_started_rx.recv().unwrap();
        assert!(enable_entered_rx
            .recv_timeout(Duration::from_millis(50))
            .is_err());
        release_disable_tx.send(()).unwrap();
        disable.join().unwrap().unwrap();
        enable.join().unwrap().unwrap();

        assert!(settings.lock().unwrap().auto_install_updates);
        assert!(updater.lock().unwrap().snapshot().auto_install_enabled);
    }

    #[test]
    fn enabling_automatic_install_while_ready_requests_a_guarded_attempt() {
        let mut session = ready_session().into_inner().unwrap();
        session.set_auto_install_enabled(false);

        let should_install = session.set_auto_install_enabled(true);

        assert!(should_install);
        assert_eq!(session.phase(), UpdatePhase::Ready);
        assert!(session.has_payload());
    }

    #[test]
    fn automatic_install_preparation_defers_without_stopping_an_active_lifecycle() {
        let receiver = Mutex::new(crate::receiver_task::ReceiverController::default());
        receiver.lock().unwrap().reserve_install().unwrap();
        let updater = ready_session();

        let preparation = prepare_install(
            &receiver,
            &updater,
            InstallMode::Automatic,
            || panic!("automatic installation must not stop the receiver"),
            |_| {},
        )
        .expect("expected automatic deferral is not an error");

        assert!(matches!(preparation, InstallPreparation::Deferred(_)));
        assert_eq!(updater.lock().unwrap().phase(), UpdatePhase::Ready);
        assert!(updater.lock().unwrap().has_payload());
    }

    #[test]
    fn automatic_install_preparation_confirms_receiver_shutdown() {
        let receiver = Mutex::new(crate::receiver_task::ReceiverController::default());
        let updater = ready_session();
        let stop_called = std::cell::Cell::new(false);

        let preparation = prepare_install(
            &receiver,
            &updater,
            InstallMode::Automatic,
            || {
                stop_called.set(true);
                StopOutcome {
                    worker_terminated: true,
                    neutral: true,
                }
            },
            |_| {},
        )
        .expect("automatic installation can be prepared after receiver shutdown");

        assert!(matches!(preparation, InstallPreparation::Proceed { .. }));
        assert!(stop_called.get());
        assert!(receiver.lock().unwrap().install_reserved());
        assert_eq!(updater.lock().unwrap().phase(), UpdatePhase::Installing);
    }

    #[test]
    fn automatic_install_preparation_requires_restart_when_neutralization_fails() {
        let receiver = Mutex::new(crate::receiver_task::ReceiverController::default());
        let updater = ready_session();

        let preparation = prepare_install(
            &receiver,
            &updater,
            InstallMode::Automatic,
            || StopOutcome {
                worker_terminated: true,
                neutral: false,
            },
            |_| {},
        )
        .expect("failed neutralization returns a blocked update");

        assert!(matches!(preparation, InstallPreparation::Blocked(_)));
        assert_eq!(
            updater.lock().unwrap().phase(),
            UpdatePhase::RestartRequired
        );
        assert!(!updater.lock().unwrap().has_payload());
        assert!(!receiver.lock().unwrap().install_reserved());

        let retry = prepare_install(
            &receiver,
            &updater,
            InstallMode::Automatic,
            || panic!("restart-required retry must not assume an absent worker is neutral"),
            |_| {},
        )
        .expect("automatic retry remains deferred until restart");

        assert!(matches!(retry, InstallPreparation::Deferred(_)));
        assert_eq!(
            updater.lock().unwrap().phase(),
            UpdatePhase::RestartRequired
        );
    }

    #[test]
    fn automatic_install_preparation_rechecks_the_preference_after_lifecycle_reservation() {
        let receiver = Mutex::new(crate::receiver_task::ReceiverController::default());
        let updater = ready_session();
        updater.lock().unwrap().set_auto_install_enabled(false);

        let preparation = prepare_install(
            &receiver,
            &updater,
            InstallMode::Automatic,
            || panic!("disabled automatic installation must not stop the receiver"),
            |_| {},
        )
        .expect("disabled automatic installation is deferred");

        assert!(matches!(preparation, InstallPreparation::Deferred(_)));
        assert!(!receiver.lock().unwrap().install_reserved());
        assert_eq!(updater.lock().unwrap().phase(), UpdatePhase::Ready);
        assert!(updater.lock().unwrap().has_payload());
    }

    #[test]
    fn ready_is_entered_only_after_verified_download_bytes_are_returned() {
        let mut session = UpdateSession::new("1.0.0".to_owned(), true, true);
        session.resource = Some(((), None));
        session.snapshot.phase = UpdatePhase::Available;
        let reservation = session
            .reserve(UpdateOperation::Download, true)
            .expect("available update can download");

        session.update_download_progress(reservation, 8, None);
        assert_eq!(session.phase(), UpdatePhase::Downloading);
        assert!(!session.has_payload());

        assert!(session.finish_download(reservation, vec![0; 8]));
        assert_eq!(session.phase(), UpdatePhase::Ready);
        assert!(session.has_payload());
    }

    #[test]
    fn every_published_session_transition_advances_the_revision() {
        let mut session = UpdateSession::<()>::new("1.0.0".to_owned(), true, true);
        let initial_revision = session.snapshot().revision;
        let reservation = session
            .reserve(UpdateOperation::Check, true)
            .expect("check reserves the session");
        let busy_revision = session.snapshot().revision;

        session.finish_failure(reservation, "synthetic offline response".to_owned());

        assert!(busy_revision > initial_revision);
        assert!(session.snapshot().revision > busy_revision);
    }
}
