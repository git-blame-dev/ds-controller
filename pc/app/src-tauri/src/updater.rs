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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationReservation(u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateSessionError {
    Busy,
    NoAvailableUpdate,
    NoReadyUpdate,
    AutomaticCheckSuppressed,
}

enum InstallPreparation<T> {
    Proceed {
        reservation: OperationReservation,
        update: T,
        bytes: Vec<u8>,
    },
    Blocked(UpdateSnapshot),
}

fn prepare_install<T>(
    receiver: &Mutex<ReceiverController>,
    updater: &Mutex<UpdateSession<T>>,
    stop_worker: impl FnOnce() -> StopOutcome,
    mut publish: impl FnMut(UpdateSnapshot),
) -> Result<InstallPreparation<T>, CommandErrorDto> {
    receiver
        .lock()
        .map_err(|_| CommandErrorDto::state_unavailable())?
        .reserve_install()
        .map_err(CommandErrorDto::updater_error)?;

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
        if outcome.worker_terminated {
            session.restore_ready(reservation, update, bytes, message);
        } else {
            let _ = (update, bytes);
            session.finish_restart_required(reservation, message);
        }
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
    pub fn new(current_version: String, auto_download_enabled: bool) -> Self {
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
            UpdatePhase::Idle
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

    pub fn finish_download(&mut self, reservation: OperationReservation, bytes: Vec<u8>) {
        if !self.finish_operation(reservation) {
            return;
        }
        if let Some((update, _)) = self.resource.take() {
            self.resource = Some((update, Some(bytes)));
            self.snapshot.phase = UpdatePhase::Ready;
        }
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
        let enabled = app
            .state::<AppState>()
            .updater()
            .lock()
            .map(|session| session.snapshot().auto_download_enabled)
            .unwrap_or(false);
        if enabled {
            let _ = perform_check(app, false).await;
        }
    });
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
    ensure_bundle_eligible()?;
    let receiver = app.state::<AppState>().receiver().clone();
    let updater_state = app.state::<AppState>().updater().clone();
    let preparation_receiver = receiver.clone();
    let preparation_updater = updater_state.clone();
    let stop_receiver = receiver.clone();
    let stop_app = app.clone();
    let publish_app = app.clone();
    let preparation = tauri::async_runtime::spawn_blocking(move || {
        prepare_install(
            &preparation_receiver,
            &preparation_updater,
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
    .await
    .map_err(|error| CommandErrorDto::updater_error(error.to_string()))?;
    let (reservation, update, bytes) = match preparation? {
        InstallPreparation::Proceed {
            reservation,
            update,
            bytes,
        } => (reservation, update, bytes),
        InstallPreparation::Blocked(snapshot) => return Ok(snapshot),
    };

    let install_result = tauri::async_runtime::spawn_blocking(move || {
        let result = update.install(&bytes).map_err(|error| error.to_string());
        (result, update, bytes)
    })
    .await
    .map_err(|error| CommandErrorDto::updater_error(error.to_string()))?;

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
            {
                let _ = (&update, &bytes);
                if let Ok(mut receiver) = receiver.lock() {
                    receiver.mark_install_failure(true);
                }
                let snapshot = {
                    let mut session = updater_state
                        .lock()
                        .map_err(|_| CommandErrorDto::state_unavailable())?;
                    session.finish_restart_required(
                        reservation,
                        format!(
                            "{message}. Restart DS Controller and check the Debian package state before using the receiver."
                        ),
                    );
                    session.snapshot()
                };
                let _ = app.emit(SNAPSHOT_EVENT, snapshot.clone());
                Ok(snapshot)
            }
            #[cfg(windows)]
            {
                if let Ok(mut receiver) = receiver.lock() {
                    receiver.mark_install_failure(false);
                }
                let snapshot = {
                    let mut session = updater_state
                        .lock()
                        .map_err(|_| CommandErrorDto::state_unavailable())?;
                    session.restore_ready(reservation, update, bytes, message);
                    session.snapshot()
                };
                let _ = app.emit(SNAPSHOT_EVENT, snapshot.clone());
                Ok(snapshot)
            }
            #[cfg(not(any(target_os = "linux", windows)))]
            {
                let _ = (message, update, bytes);
                Err(CommandErrorDto::updater_error(
                    "unsupported update platform",
                ))
            }
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

    let snapshot = {
        let mut session = updater_state
            .lock()
            .map_err(|_| CommandErrorDto::state_unavailable())?;
        match downloaded {
            Ok(bytes) => session.finish_download(reservation, bytes),
            Err(error) => session.finish_failure(reservation, error.to_string()),
        }
        session.snapshot()
    };
    let _ = app.emit(SNAPSHOT_EVENT, snapshot.clone());
    Ok(snapshot)
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
        let mut session = UpdateSession::new("1.0.0".to_owned(), true);
        session.resource = Some(((), Some(vec![1, 2, 3])));
        session.snapshot.phase = UpdatePhase::Ready;
        Mutex::new(session)
    }

    #[test]
    fn missing_release_metadata_is_unavailable_instead_of_an_update_error() {
        let mut session = UpdateSession::<()>::new("1.0.0".to_owned(), true);
        let reservation = session
            .reserve(UpdateOperation::Check, false)
            .expect("check reserves the session");

        session.finish_unavailable(reservation);

        assert_eq!(session.phase(), UpdatePhase::Unavailable);
        assert_eq!(session.snapshot().error, None);
    }

    #[test]
    fn install_preparation_proceeds_after_reserving_and_stopping_neutral_worker() {
        let receiver = Mutex::new(crate::receiver_task::ReceiverController::default());
        let updater = ready_session();

        let preparation = prepare_install(
            &receiver,
            &updater,
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
    fn install_preparation_restores_update_and_lifecycle_after_neutral_failure() {
        let receiver = Mutex::new(crate::receiver_task::ReceiverController::default());
        let updater = ready_session();

        let preparation =
            prepare_install(&receiver, &updater, || stop_outcome(true, false), |_| {})
                .expect("known termination is handled");

        assert!(matches!(preparation, InstallPreparation::Blocked(_)));
        assert!(receiver.lock().unwrap().reserve_install().is_ok());
        let session = updater.lock().unwrap();
        assert_eq!(session.phase(), UpdatePhase::Ready);
        assert!(session.has_payload());
    }

    #[test]
    fn install_preparation_requires_restart_after_unknown_worker_termination() {
        let receiver = Mutex::new(crate::receiver_task::ReceiverController::default());
        let updater = ready_session();

        let preparation =
            prepare_install(&receiver, &updater, || stop_outcome(false, false), |_| {})
                .expect("unknown termination is handled");

        assert!(matches!(preparation, InstallPreparation::Blocked(_)));
        assert!(receiver.lock().unwrap().reserve_install().is_err());
        let session = updater.lock().unwrap();
        assert_eq!(session.phase(), UpdatePhase::RestartRequired);
        assert!(!session.has_payload());
    }

    #[test]
    fn later_discards_ready_payload_and_suppresses_automatic_rediscovery() {
        let mut session = UpdateSession::new("1.0.0".to_owned(), true);
        session.resource = Some(((), Some(vec![1, 2, 3])));
        session.snapshot.phase = UpdatePhase::Ready;

        session.defer().expect("ready update can be deferred");

        assert_eq!(session.phase(), UpdatePhase::Idle);
        assert!(!session.has_payload());
        assert!(session.automatic_checks_suppressed());
    }

    #[test]
    fn busy_operation_rejects_another_operation_without_queueing() {
        let mut session = UpdateSession::<()>::new("1.0.0".to_owned(), true);
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
    fn ready_is_entered_only_after_verified_download_bytes_are_returned() {
        let mut session = UpdateSession::new("1.0.0".to_owned(), true);
        session.resource = Some(((), None));
        session.snapshot.phase = UpdatePhase::Available;
        let reservation = session
            .reserve(UpdateOperation::Download, true)
            .expect("available update can download");

        session.update_download_progress(reservation, 8, None);
        assert_eq!(session.phase(), UpdatePhase::Downloading);
        assert!(!session.has_payload());

        session.finish_download(reservation, vec![0; 8]);
        assert_eq!(session.phase(), UpdatePhase::Ready);
        assert!(session.has_payload());
    }

    #[test]
    fn every_published_session_transition_advances_the_revision() {
        let mut session = UpdateSession::<()>::new("1.0.0".to_owned(), true);
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
