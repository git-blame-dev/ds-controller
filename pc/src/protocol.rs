use std::fmt;

pub const PACKET_SIZE: usize = 16;
const MAGIC: &[u8; 4] = b"DSCP";
const VERSION: u8 = 1;
const MESSAGE_TYPE_CONTROLLER_STATE: u8 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControllerState {
    pub sequence: u32,
    pub buttons: Buttons,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Buttons(u16);

impl Buttons {
    pub const A: Self = Self(1 << 0);
    pub const B: Self = Self(1 << 1);
    pub const X: Self = Self(1 << 2);
    pub const Y: Self = Self(1 << 3);
    pub const L: Self = Self(1 << 4);
    pub const R: Self = Self(1 << 5);
    pub const START: Self = Self(1 << 6);
    pub const SELECT: Self = Self(1 << 7);
    pub const DPAD_UP: Self = Self(1 << 8);
    pub const DPAD_DOWN: Self = Self(1 << 9);
    pub const DPAD_LEFT: Self = Self(1 << 10);
    pub const DPAD_RIGHT: Self = Self(1 << 11);

    const KNOWN_MASK: u16 = Self::A.0
        | Self::B.0
        | Self::X.0
        | Self::Y.0
        | Self::L.0
        | Self::R.0
        | Self::START.0
        | Self::SELECT.0
        | Self::DPAD_UP.0
        | Self::DPAD_DOWN.0
        | Self::DPAD_LEFT.0
        | Self::DPAD_RIGHT.0;

    pub fn from_bits_truncate(bits: u16) -> Self {
        Self(bits & Self::KNOWN_MASK)
    }

    pub fn contains(self, button: Self) -> bool {
        self.0 & button.0 != 0
    }

    #[cfg(test)]
    pub fn raw(self) -> u16 {
        self.0
    }
}

impl fmt::Display for Buttons {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names = [
            (Self::A, "A"),
            (Self::B, "B"),
            (Self::X, "X"),
            (Self::Y, "Y"),
            (Self::L, "L"),
            (Self::R, "R"),
            (Self::START, "Start"),
            (Self::SELECT, "Select"),
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

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    WrongSize { actual: usize },
    WrongMagic,
    UnsupportedVersion(u8),
    UnsupportedMessageType(u8),
    WrongPacketSizeField(u16),
    NonZeroFlags(u8),
    NonZeroReserved(u8),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongSize { actual } => write!(f, "wrong packet size: {actual}"),
            Self::WrongMagic => write!(f, "wrong packet magic"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported protocol version: {version}")
            }
            Self::UnsupportedMessageType(message_type) => {
                write!(f, "unsupported message type: {message_type}")
            }
            Self::WrongPacketSizeField(size) => write!(f, "wrong packet size field: {size}"),
            Self::NonZeroFlags(flags) => write!(f, "non-zero flags byte: {flags}"),
            Self::NonZeroReserved(reserved) => write!(f, "non-zero reserved byte: {reserved}"),
        }
    }
}

impl std::error::Error for ParseError {}

pub fn parse_controller_state(packet: &[u8]) -> Result<ControllerState, ParseError> {
    if packet.len() != PACKET_SIZE {
        return Err(ParseError::WrongSize {
            actual: packet.len(),
        });
    }

    if &packet[0..4] != MAGIC {
        return Err(ParseError::WrongMagic);
    }

    if packet[4] != VERSION {
        return Err(ParseError::UnsupportedVersion(packet[4]));
    }

    if packet[5] != MESSAGE_TYPE_CONTROLLER_STATE {
        return Err(ParseError::UnsupportedMessageType(packet[5]));
    }

    let packet_size = u16::from_le_bytes([packet[6], packet[7]]);
    if packet_size != PACKET_SIZE as u16 {
        return Err(ParseError::WrongPacketSizeField(packet_size));
    }

    let sequence = u32::from_le_bytes([packet[8], packet[9], packet[10], packet[11]]);
    let button_bits = u16::from_le_bytes([packet[12], packet[13]]);

    if packet[14] != 0 {
        return Err(ParseError::NonZeroFlags(packet[14]));
    }

    if packet[15] != 0 {
        return Err(ParseError::NonZeroReserved(packet[15]));
    }

    Ok(ControllerState {
        sequence,
        buttons: Buttons::from_bits_truncate(button_bits),
    })
}

#[cfg(test)]
pub fn encode_controller_state_for_test(state: ControllerState) -> [u8; PACKET_SIZE] {
    let mut packet = [0u8; PACKET_SIZE];
    packet[0..4].copy_from_slice(MAGIC);
    packet[4] = VERSION;
    packet[5] = MESSAGE_TYPE_CONTROLLER_STATE;
    packet[6..8].copy_from_slice(&(PACKET_SIZE as u16).to_le_bytes());
    packet[8..12].copy_from_slice(&state.sequence.to_le_bytes());
    packet[12..14].copy_from_slice(&state.buttons.raw().to_le_bytes());
    packet
}

pub fn is_newer_sequence(candidate: u32, current: u32) -> bool {
    candidate != current && candidate.wrapping_sub(current) < 0x8000_0000
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(sequence: u32, buttons: u16) -> [u8; PACKET_SIZE] {
        let mut packet = [0u8; PACKET_SIZE];
        packet[0..4].copy_from_slice(MAGIC);
        packet[4] = VERSION;
        packet[5] = MESSAGE_TYPE_CONTROLLER_STATE;
        packet[6..8].copy_from_slice(&(PACKET_SIZE as u16).to_le_bytes());
        packet[8..12].copy_from_slice(&sequence.to_le_bytes());
        packet[12..14].copy_from_slice(&buttons.to_le_bytes());
        packet
    }

    #[test]
    fn encodes_golden_controller_state_packet() {
        let packet = encode_controller_state_for_test(ControllerState {
            sequence: 42,
            buttons: Buttons::from_bits_truncate(Buttons::A.raw() | Buttons::DPAD_UP.raw()),
        });

        assert_eq!(
            packet,
            [b'D', b'S', b'C', b'P', 1, 1, 16, 0, 42, 0, 0, 0, 1, 1, 0, 0,]
        );
    }

    #[test]
    fn parses_valid_controller_state() {
        let state = parse_controller_state(&packet(42, Buttons::A.0 | Buttons::DPAD_UP.0))
            .expect("valid packet");

        assert_eq!(state.sequence, 42);
        assert!(state.buttons.contains(Buttons::A));
        assert!(state.buttons.contains(Buttons::DPAD_UP));
        assert!(!state.buttons.contains(Buttons::B));
    }

    #[test]
    fn rejects_truncated_packet() {
        assert_eq!(
            parse_controller_state(&packet(0, 0)[..PACKET_SIZE - 1]),
            Err(ParseError::WrongSize {
                actual: PACKET_SIZE - 1
            })
        );
    }

    #[test]
    fn rejects_wrong_magic() {
        let mut packet = packet(0, 0);
        packet[0] = b'X';

        assert_eq!(parse_controller_state(&packet), Err(ParseError::WrongMagic));
    }

    #[test]
    fn rejects_wrong_version() {
        let mut packet = packet(0, 0);
        packet[4] = 2;

        assert_eq!(
            parse_controller_state(&packet),
            Err(ParseError::UnsupportedVersion(2))
        );
    }

    #[test]
    fn rejects_wrong_message_type() {
        let mut packet = packet(0, 0);
        packet[5] = 9;

        assert_eq!(
            parse_controller_state(&packet),
            Err(ParseError::UnsupportedMessageType(9))
        );
    }

    #[test]
    fn rejects_wrong_size_field() {
        let mut packet = packet(0, 0);
        packet[6..8].copy_from_slice(&20u16.to_le_bytes());

        assert_eq!(
            parse_controller_state(&packet),
            Err(ParseError::WrongPacketSizeField(20))
        );
    }

    #[test]
    fn rejects_non_zero_flags() {
        let mut packet = packet(0, 0);
        packet[14] = 1;

        assert_eq!(
            parse_controller_state(&packet),
            Err(ParseError::NonZeroFlags(1))
        );
    }

    #[test]
    fn rejects_non_zero_reserved() {
        let mut packet = packet(0, 0);
        packet[15] = 1;

        assert_eq!(
            parse_controller_state(&packet),
            Err(ParseError::NonZeroReserved(1))
        );
    }

    #[test]
    fn truncates_unknown_button_bits() {
        let state = parse_controller_state(&packet(0, 0xffff)).expect("valid packet");

        assert_eq!(state.buttons.0, Buttons::KNOWN_MASK);
    }

    #[test]
    fn detects_newer_sequence_numbers() {
        assert!(is_newer_sequence(2, 1));
        assert!(!is_newer_sequence(1, 1));
        assert!(!is_newer_sequence(1, 2));
    }

    #[test]
    fn detects_wrapped_sequence_numbers() {
        assert!(is_newer_sequence(0, u32::MAX));
        assert!(!is_newer_sequence(u32::MAX, 0));
    }
}
