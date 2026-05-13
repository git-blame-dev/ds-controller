use std::fmt;

use crate::protocol::{Buttons, ControllerState};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ControllerOutputState {
    pub buttons: XboxButtons,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct XboxButtons(u16);

impl XboxButtons {
    pub const DPAD_UP: Self = Self(1 << 0);
    pub const DPAD_DOWN: Self = Self(1 << 1);
    pub const DPAD_LEFT: Self = Self(1 << 2);
    pub const DPAD_RIGHT: Self = Self(1 << 3);
    pub const START: Self = Self(1 << 4);
    pub const BACK: Self = Self(1 << 5);
    pub const LEFT_BUMPER: Self = Self(1 << 8);
    pub const RIGHT_BUMPER: Self = Self(1 << 9);
    pub const A: Self = Self(1 << 12);
    pub const B: Self = Self(1 << 13);
    pub const X: Self = Self(1 << 14);
    pub const Y: Self = Self(1 << 15);

    #[cfg(any(test, windows))]
    pub fn bits(self) -> u16 {
        self.0
    }

    fn insert(&mut self, button: Self) {
        self.0 |= button.0;
    }

    pub fn contains(self, button: Self) -> bool {
        self.0 & button.0 != 0
    }
}

impl fmt::Display for XboxButtons {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names = [
            (Self::A, "A"),
            (Self::B, "B"),
            (Self::X, "X"),
            (Self::Y, "Y"),
            (Self::LEFT_BUMPER, "LB"),
            (Self::RIGHT_BUMPER, "RB"),
            (Self::START, "Start"),
            (Self::BACK, "Back"),
            (Self::DPAD_UP, "Up"),
            (Self::DPAD_DOWN, "Down"),
            (Self::DPAD_LEFT, "Left"),
            (Self::DPAD_RIGHT, "Right"),
        ];

        let mut wrote_any = false;
        for (button, name) in names {
            if self.contains(button) {
                if wrote_any {
                    write!(f, "+")?;
                }
                write!(f, "{name}")?;
                wrote_any = true;
            }
        }

        if !wrote_any {
            write!(f, "neutral")?;
        }

        Ok(())
    }
}

pub fn map_ds_to_xbox(state: ControllerState) -> ControllerOutputState {
    let mut buttons = XboxButtons::default();

    map_button(&mut buttons, state.buttons, Buttons::A, XboxButtons::A);
    map_button(&mut buttons, state.buttons, Buttons::B, XboxButtons::B);
    map_button(&mut buttons, state.buttons, Buttons::X, XboxButtons::X);
    map_button(&mut buttons, state.buttons, Buttons::Y, XboxButtons::Y);
    map_button(
        &mut buttons,
        state.buttons,
        Buttons::L,
        XboxButtons::LEFT_BUMPER,
    );
    map_button(
        &mut buttons,
        state.buttons,
        Buttons::R,
        XboxButtons::RIGHT_BUMPER,
    );
    map_button(
        &mut buttons,
        state.buttons,
        Buttons::START,
        XboxButtons::START,
    );
    map_button(
        &mut buttons,
        state.buttons,
        Buttons::SELECT,
        XboxButtons::BACK,
    );
    map_button(
        &mut buttons,
        state.buttons,
        Buttons::DPAD_UP,
        XboxButtons::DPAD_UP,
    );
    map_button(
        &mut buttons,
        state.buttons,
        Buttons::DPAD_DOWN,
        XboxButtons::DPAD_DOWN,
    );
    map_button(
        &mut buttons,
        state.buttons,
        Buttons::DPAD_LEFT,
        XboxButtons::DPAD_LEFT,
    );
    map_button(
        &mut buttons,
        state.buttons,
        Buttons::DPAD_RIGHT,
        XboxButtons::DPAD_RIGHT,
    );

    ControllerOutputState { buttons }
}

fn map_button(buttons: &mut XboxButtons, ds_buttons: Buttons, ds: Buttons, xbox: XboxButtons) {
    if ds_buttons.contains(ds) {
        buttons.insert(xbox);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_neutral_state() {
        let output = map_ds_to_xbox(ControllerState {
            sequence: 1,
            buttons: Buttons::default(),
        });

        assert_eq!(output, ControllerOutputState::default());
    }

    #[test]
    fn maps_each_ds_button_to_exact_xbox_button() {
        let cases = [
            (Buttons::A, XboxButtons::A),
            (Buttons::B, XboxButtons::B),
            (Buttons::X, XboxButtons::X),
            (Buttons::Y, XboxButtons::Y),
            (Buttons::L, XboxButtons::LEFT_BUMPER),
            (Buttons::R, XboxButtons::RIGHT_BUMPER),
            (Buttons::START, XboxButtons::START),
            (Buttons::SELECT, XboxButtons::BACK),
            (Buttons::DPAD_UP, XboxButtons::DPAD_UP),
            (Buttons::DPAD_DOWN, XboxButtons::DPAD_DOWN),
            (Buttons::DPAD_LEFT, XboxButtons::DPAD_LEFT),
            (Buttons::DPAD_RIGHT, XboxButtons::DPAD_RIGHT),
        ];

        for (ds_button, xbox_button) in cases {
            let output = map_ds_to_xbox(ControllerState {
                sequence: 1,
                buttons: Buttons::from_bits_truncate(ds_button.raw()),
            });

            assert_eq!(output.buttons.bits(), xbox_button.bits());
        }
    }

    #[test]
    fn maps_multiple_buttons_without_extra_outputs() {
        let output = map_ds_to_xbox(ControllerState {
            sequence: 1,
            buttons: Buttons::from_bits_truncate(
                Buttons::A.raw()
                    | Buttons::L.raw()
                    | Buttons::START.raw()
                    | Buttons::DPAD_RIGHT.raw(),
            ),
        });

        assert_eq!(
            output.buttons.bits(),
            XboxButtons::A.bits()
                | XboxButtons::LEFT_BUMPER.bits()
                | XboxButtons::START.bits()
                | XboxButtons::DPAD_RIGHT.bits()
        );
    }
}
