use std::io;

use evdev::{
    uinput::VirtualDevice, AttributeSet, BusType, EventType, InputEvent, InputId, KeyCode,
};

use super::{BackendError, ControllerBackend};
use crate::mapping::{ControllerOutputState, XboxButtons};

const BUTTON_MAP: [(XboxButtons, KeyCode); 12] = [
    (XboxButtons::A, KeyCode::BTN_SOUTH),
    (XboxButtons::B, KeyCode::BTN_EAST),
    (XboxButtons::X, KeyCode::BTN_WEST),
    (XboxButtons::Y, KeyCode::BTN_NORTH),
    (XboxButtons::LEFT_BUMPER, KeyCode::BTN_TL),
    (XboxButtons::RIGHT_BUMPER, KeyCode::BTN_TR),
    (XboxButtons::START, KeyCode::BTN_START),
    (XboxButtons::BACK, KeyCode::BTN_SELECT),
    (XboxButtons::DPAD_UP, KeyCode::BTN_DPAD_UP),
    (XboxButtons::DPAD_DOWN, KeyCode::BTN_DPAD_DOWN),
    (XboxButtons::DPAD_LEFT, KeyCode::BTN_DPAD_LEFT),
    (XboxButtons::DPAD_RIGHT, KeyCode::BTN_DPAD_RIGHT),
];

pub struct UinputBackend {
    device: VirtualDevice,
}

impl UinputBackend {
    pub fn new() -> Result<Self, BackendError> {
        let keys = BUTTON_MAP
            .iter()
            .map(|(_, key)| *key)
            .collect::<AttributeSet<_>>();
        let device = VirtualDevice::builder()
            .and_then(|builder| {
                builder
                    .name("DS Controller Virtual Gamepad")
                    .input_id(InputId::new(BusType::BUS_VIRTUAL, 0, 0, 1))
                    .with_keys(&keys)?
                    .build()
            })
            .map_err(|error| uinput_error("create virtual gamepad", error))?;
        let mut backend = Self { device };
        backend.update(ControllerOutputState::default())?;
        Ok(backend)
    }
}

impl ControllerBackend for UinputBackend {
    fn update(&mut self, state: ControllerOutputState) -> Result<(), BackendError> {
        self.device
            .emit(&button_events(state))
            .map_err(|error| uinput_error("update virtual gamepad", error))
    }
}

impl Drop for UinputBackend {
    fn drop(&mut self) {
        let _ = self.update(ControllerOutputState::default());
    }
}

fn button_events(state: ControllerOutputState) -> Vec<InputEvent> {
    BUTTON_MAP
        .iter()
        .map(|(button, key)| {
            InputEvent::new(
                EventType::KEY.0,
                key.code(),
                i32::from(state.buttons.contains(*button)),
            )
        })
        .collect()
}

fn uinput_error(action: &str, error: io::Error) -> BackendError {
    if error.kind() == io::ErrorKind::PermissionDenied {
        return BackendError::new(format!(
            "cannot access /dev/uinput; install the DS Controller Debian package to grant the active user permission ({error})"
        ));
    }

    BackendError::new(format!("failed to {action}: {error}"))
}

#[cfg(test)]
mod tests {
    use crate::{
        mapping::map_ds_to_xbox,
        protocol::{Buttons, ControllerState},
    };

    use super::*;

    #[test]
    fn neutral_state_releases_every_linux_button() {
        let events = button_events(ControllerOutputState::default());

        assert_eq!(events.len(), BUTTON_MAP.len());
        for (event, (_, key)) in events.iter().zip(BUTTON_MAP) {
            assert_eq!(event.event_type(), EventType::KEY);
            assert_eq!(event.code(), key.code());
            assert_eq!(event.value(), 0);
        }
    }

    #[test]
    fn maps_each_xbox_button_to_one_linux_gamepad_button() {
        for (pressed_button, expected_key) in BUTTON_MAP {
            let events = button_events(ControllerOutputState {
                buttons: pressed_button,
            });

            assert_eq!(events.len(), BUTTON_MAP.len());
            for (event, (_, key)) in events.iter().zip(BUTTON_MAP) {
                let expected_value = i32::from(key == expected_key);
                assert_eq!(event.code(), key.code());
                assert_eq!(event.value(), expected_value);
            }
        }
    }

    #[test]
    fn maps_combined_buttons_without_extra_presses() {
        let state = map_ds_to_xbox(ControllerState {
            sequence: 1,
            buttons: Buttons::from_bits_truncate((1 << 0) | (1 << 4) | (1 << 6) | (1 << 11)),
        });
        let events = button_events(state);
        let pressed_codes = events
            .iter()
            .filter(|event| event.value() == 1)
            .map(InputEvent::code)
            .collect::<Vec<_>>();

        assert_eq!(
            pressed_codes,
            vec![
                KeyCode::BTN_SOUTH.code(),
                KeyCode::BTN_TL.code(),
                KeyCode::BTN_START.code(),
                KeyCode::BTN_DPAD_RIGHT.code(),
            ]
        );
    }
}
