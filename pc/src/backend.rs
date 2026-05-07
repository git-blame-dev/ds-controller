use std::fmt;

use crate::mapping::ControllerOutputState;

#[cfg(windows)]
mod vigem;

pub trait ControllerBackend {
    fn update(&mut self, state: ControllerOutputState) -> Result<(), BackendError>;

    fn neutral(&mut self) -> Result<(), BackendError> {
        self.update(ControllerOutputState::default())
    }
}

#[derive(Debug)]
pub struct BackendError(String);

impl BackendError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for BackendError {}

pub fn create_backend(no_vigem: bool) -> Result<Box<dyn ControllerBackend>, BackendError> {
    if no_vigem {
        return Ok(Box::new(NoopBackend));
    }

    create_platform_backend()
}

#[cfg(windows)]
fn create_platform_backend() -> Result<Box<dyn ControllerBackend>, BackendError> {
    Ok(Box::new(vigem::VigemBackend::new()?))
}

#[cfg(not(windows))]
fn create_platform_backend() -> Result<Box<dyn ControllerBackend>, BackendError> {
    Err(BackendError::new(
        "ViGEm output requires Windows; use --no-vigem on this platform",
    ))
}

struct NoopBackend;

impl ControllerBackend for NoopBackend {
    fn update(&mut self, _state: ControllerOutputState) -> Result<(), BackendError> {
        Ok(())
    }
}

#[cfg(windows)]
impl From<vigem_client::Error> for BackendError {
    fn from(error: vigem_client::Error) -> Self {
        Self::new(format!("ViGEm error: {error:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_usable_noop_backend() {
        let mut backend = create_backend(true).expect("noop backend");

        assert!(backend.update(ControllerOutputState::default()).is_ok());
        assert!(backend.neutral().is_ok());
    }

    #[test]
    #[cfg(not(windows))]
    fn returns_clear_error_for_vigem_on_non_windows() {
        let error = match create_backend(false) {
            Ok(_) => panic!("expected non-windows vigem error"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("ViGEm output requires Windows"));
    }

    #[test]
    fn neutral_sends_default_state() {
        let mut backend = RecordingBackend { states: Vec::new() };

        backend.neutral().expect("neutral update");

        assert_eq!(backend.states, vec![ControllerOutputState::default()]);
    }

    struct RecordingBackend {
        states: Vec<ControllerOutputState>,
    }

    impl ControllerBackend for RecordingBackend {
        fn update(&mut self, state: ControllerOutputState) -> Result<(), BackendError> {
            self.states.push(state);
            Ok(())
        }
    }
}
