use std::fmt;
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

use crate::protocol::{self, ControllerState, PACKET_SIZE};

#[derive(Clone, Copy, Debug)]
pub struct ReceiverConfig {
    pub bind_addr: SocketAddr,
    pub timeout: Duration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceiverEvent {
    State {
        sender: SocketAddr,
        state: ControllerState,
    },
    Timeout,
}

#[derive(Debug)]
pub enum ReceiverError {
    Io(io::Error),
    Parse(protocol::ParseError),
}

impl fmt::Display for ReceiverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "I/O error: {error}"),
            Self::Parse(error) => write!(f, "parse error: {error}"),
        }
    }
}

impl std::error::Error for ReceiverError {}

impl From<io::Error> for ReceiverError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<protocol::ParseError> for ReceiverError {
    fn from(error: protocol::ParseError) -> Self {
        Self::Parse(error)
    }
}

pub struct Receiver {
    socket: UdpSocket,
    latest_sequence: Option<u32>,
}

impl Receiver {
    pub fn bind(config: ReceiverConfig) -> Result<Self, io::Error> {
        let socket = UdpSocket::bind(config.bind_addr)?;
        socket.set_read_timeout(Some(config.timeout))?;

        Ok(Self {
            socket,
            latest_sequence: None,
        })
    }

    pub fn next_event(&mut self) -> Result<ReceiverEvent, ReceiverError> {
        let mut buffer = [0u8; PACKET_SIZE];

        loop {
            match self.socket.recv_from(&mut buffer) {
                Ok((len, sender)) => {
                    let state = protocol::parse_controller_state(&buffer[..len])?;

                    if !self.accept_sequence(state.sequence) {
                        continue;
                    }

                    return Ok(ReceiverEvent::State { sender, state });
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                    ) =>
                {
                    self.latest_sequence = None;
                    return Ok(ReceiverEvent::Timeout);
                }
                Err(error) => return Err(error.into()),
            }
        }
    }

    fn accept_sequence(&mut self, sequence: u32) -> bool {
        match self.latest_sequence {
            Some(latest_sequence) if !protocol::is_newer_sequence(sequence, latest_sequence) => {
                false
            }
            _ => {
                self.latest_sequence = Some(sequence);
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddrV4};
    use std::time::Duration;

    use super::*;
    use crate::protocol::{encode_controller_state_for_test, Buttons};

    fn localhost(port: u16) -> SocketAddr {
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port))
    }

    fn receiver_config() -> ReceiverConfig {
        ReceiverConfig {
            bind_addr: localhost(0),
            timeout: Duration::from_millis(20),
        }
    }

    fn bind_receiver(config: ReceiverConfig) -> Receiver {
        Receiver::bind(config).expect("receiver binds")
    }

    fn state(sequence: u32, buttons: Buttons) -> ControllerState {
        ControllerState { sequence, buttons }
    }

    fn send_state(sender: &UdpSocket, receiver: &Receiver, state: ControllerState) {
        sender
            .send_to(
                &encode_controller_state_for_test(state),
                receiver.socket.local_addr().expect("receiver address"),
            )
            .expect("packet sends");
    }

    fn bind_sender() -> UdpSocket {
        UdpSocket::bind(localhost(0)).expect("sender binds")
    }

    #[test]
    fn accepts_valid_packet_from_any_sender_by_default() {
        let mut receiver = bind_receiver(receiver_config());
        let sender = bind_sender();
        let expected = state(1, Buttons::A);

        send_state(&sender, &receiver, expected);

        assert_eq!(
            receiver.next_event().expect("state event"),
            ReceiverEvent::State {
                sender: sender.local_addr().expect("sender address"),
                state: expected,
            }
        );
    }

    #[test]
    fn accepts_valid_packets_from_different_senders() {
        let mut receiver = bind_receiver(receiver_config());
        let first_sender = bind_sender();
        let second_sender = bind_sender();
        let first = state(1, Buttons::A);
        let second = state(2, Buttons::B);

        send_state(&first_sender, &receiver, first);
        assert_eq!(
            receiver.next_event().expect("first state"),
            ReceiverEvent::State {
                sender: first_sender.local_addr().expect("first address"),
                state: first,
            }
        );

        send_state(&second_sender, &receiver, second);
        assert_eq!(
            receiver.next_event().expect("second state"),
            ReceiverEvent::State {
                sender: second_sender.local_addr().expect("second address"),
                state: second,
            }
        );
    }

    #[test]
    fn ignores_duplicate_and_stale_sequences_until_newer_packet_arrives() {
        let mut receiver = bind_receiver(receiver_config());
        let sender = bind_sender();
        let first = state(10, Buttons::A);
        let newer = state(11, Buttons::X);

        send_state(&sender, &receiver, first);
        assert_eq!(
            receiver.next_event().expect("first state"),
            ReceiverEvent::State {
                sender: sender.local_addr().expect("sender address"),
                state: first,
            }
        );

        send_state(&sender, &receiver, state(10, Buttons::B));
        send_state(&sender, &receiver, state(9, Buttons::Y));
        send_state(&sender, &receiver, newer);

        assert_eq!(
            receiver.next_event().expect("newer state"),
            ReceiverEvent::State {
                sender: sender.local_addr().expect("sender address"),
                state: newer,
            }
        );
    }

    #[test]
    fn accepts_wrapped_newer_sequence() {
        let mut receiver = bind_receiver(receiver_config());
        let sender = bind_sender();
        let first = state(u32::MAX, Buttons::A);
        let wrapped = state(0, Buttons::B);

        send_state(&sender, &receiver, first);
        assert!(matches!(
            receiver.next_event().expect("first state"),
            ReceiverEvent::State { state, .. } if state == first
        ));

        send_state(&sender, &receiver, wrapped);
        assert!(matches!(
            receiver.next_event().expect("wrapped state"),
            ReceiverEvent::State { state, .. } if state == wrapped
        ));
    }

    #[test]
    fn returns_parse_error_for_malformed_packet() {
        let mut receiver = bind_receiver(receiver_config());
        let sender = bind_sender();

        sender
            .send_to(
                &[0; 4],
                receiver.socket.local_addr().expect("receiver address"),
            )
            .expect("malformed packet sends");

        assert!(matches!(
            receiver.next_event(),
            Err(ReceiverError::Parse(_))
        ));
    }

    #[test]
    fn emits_repeated_timeouts_then_resumes_on_valid_packet() {
        let mut receiver = bind_receiver(receiver_config());
        let sender = bind_sender();
        let expected = state(1, Buttons::START);

        assert_eq!(
            receiver.next_event().expect("timeout event"),
            ReceiverEvent::Timeout
        );
        assert_eq!(
            receiver.next_event().expect("second timeout event"),
            ReceiverEvent::Timeout
        );

        send_state(&sender, &receiver, expected);
        assert_eq!(
            receiver.next_event().expect("state event"),
            ReceiverEvent::State {
                sender: sender.local_addr().expect("sender address"),
                state: expected,
            }
        );
    }

    #[test]
    fn timeout_resets_sequence_tracking() {
        let mut receiver = bind_receiver(receiver_config());
        let sender = bind_sender();
        let first = state(5, Buttons::A);
        let after_timeout_same_sequence = state(5, Buttons::B);

        send_state(&sender, &receiver, first);
        assert!(matches!(
            receiver.next_event().expect("first state"),
            ReceiverEvent::State { state, .. } if state == first
        ));

        assert_eq!(
            receiver.next_event().expect("timeout event"),
            ReceiverEvent::Timeout
        );

        send_state(&sender, &receiver, after_timeout_same_sequence);
        assert!(matches!(
            receiver.next_event().expect("state after timeout"),
            ReceiverEvent::State { state, .. } if state == after_timeout_same_sequence
        ));
    }
}
