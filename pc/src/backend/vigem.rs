use crate::backend::{BackendError, ControllerBackend};
use crate::mapping::ControllerOutputState;

pub struct VigemBackend {
    target: vigem_client::Xbox360Wired<vigem_client::Client>,
}

impl VigemBackend {
    pub fn new() -> Result<Self, BackendError> {
        let client = vigem_client::Client::connect()?;
        let mut target =
            vigem_client::Xbox360Wired::new(client, vigem_client::TargetId::XBOX360_WIRED);

        target.plugin()?;
        target.wait_ready()?;
        target.update(&vigem_client::XGamepad::default())?;

        Ok(Self { target })
    }
}

impl ControllerBackend for VigemBackend {
    fn update(&mut self, state: ControllerOutputState) -> Result<(), BackendError> {
        let report = to_x_gamepad(state);

        self.target.update(&report)?;
        Ok(())
    }
}

impl Drop for VigemBackend {
    fn drop(&mut self) {
        let _ = self.update(ControllerOutputState::default());
    }
}

fn to_x_gamepad(state: ControllerOutputState) -> vigem_client::XGamepad {
    vigem_client::XGamepad {
        buttons: vigem_client::XButtons(state.buttons.bits()),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use crate::mapping::XboxButtons;

    use super::*;

    #[test]
    fn converts_output_buttons_to_vigem_report_bits() {
        let report = to_x_gamepad(ControllerOutputState {
            buttons: XboxButtons::A,
        });

        assert_eq!(report.buttons.raw, XboxButtons::A.bits());
    }
}
