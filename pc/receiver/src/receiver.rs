use std::fmt;
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

use crate::protocol::{self, ControllerState, PACKET_SIZE};

#[derive(Clone, Copy, Debug)]
pub struct ReceiverConfig {
    pub bind_addr: SocketAddr,
    pub timeout: Duration,
    pub fixed_sender: Option<SocketAddr>,
    pub accept_first_sender: bool,
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
    config: ReceiverConfig,
    active_sender: Option<SocketAddr>,
    latest_sequence: Option<u32>,
    timed_out: bool,
}

impl Receiver {
    pub fn bind(config: ReceiverConfig) -> Result<Self, io::Error> {
        let socket = UdpSocket::bind(config.bind_addr)?;
        socket.set_read_timeout(Some(config.timeout))?;

        Ok(Self {
            socket,
            config,
            active_sender: config.fixed_sender,
            latest_sequence: None,
            timed_out: false,
        })
    }

    pub fn next_event(&mut self) -> Result<ReceiverEvent, ReceiverError> {
        let mut buffer = [0u8; PACKET_SIZE];

        loop {
            match self.socket.recv_from(&mut buffer) {
                Ok((len, sender)) => {
                    if !self.should_parse_sender(sender) {
                        continue;
                    }

                    let state = protocol::parse_controller_state(&buffer[..len])?;

                    if !self.accept_sender(sender) {
                        continue;
                    }

                    if !self.accept_sequence(state.sequence) {
                        continue;
                    }

                    self.timed_out = false;
                    return Ok(ReceiverEvent::State { sender, state });
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                    ) =>
                {
                    if self.timed_out {
                        continue;
                    }

                    self.timed_out = true;
                    self.latest_sequence = None;
                    return Ok(ReceiverEvent::Timeout);
                }
                Err(error) => return Err(error.into()),
            }
        }
    }

    fn should_parse_sender(&self, sender: SocketAddr) -> bool {
        match self.active_sender {
            Some(active_sender) => sender == active_sender,
            None => true,
        }
    }

    fn accept_sender(&mut self, sender: SocketAddr) -> bool {
        match self.active_sender {
            Some(active_sender) => sender == active_sender,
            None if self.config.accept_first_sender => {
                self.active_sender = Some(sender);
                true
            }
            None => true,
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
            fixed_sender: None,
            accept_first_sender: false,
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
    fn filters_to_fixed_sender() {
        let accepted_sender = bind_sender();
        let ignored_sender = bind_sender();
        let mut config = receiver_config();
        config.fixed_sender = Some(accepted_sender.local_addr().expect("accepted address"));
        let mut receiver = bind_receiver(config);
        let expected = state(2, Buttons::B);

        send_state(&ignored_sender, &receiver, state(1, Buttons::A));
        send_state(&accepted_sender, &receiver, expected);

        assert_eq!(
            receiver.next_event().expect("state event"),
            ReceiverEvent::State {
                sender: accepted_sender.local_addr().expect("accepted address"),
                state: expected,
            }
        );
    }

    #[test]
    fn accept_first_sender_locks_after_valid_packet() {
        let first_sender = bind_sender();
        let ignored_sender = bind_sender();
        let mut config = receiver_config();
        config.accept_first_sender = true;
        let mut receiver = bind_receiver(config);
        let first = state(1, Buttons::A);
        let second = state(3, Buttons::X);

        send_state(&first_sender, &receiver, first);
        assert_eq!(
            receiver.next_event().expect("first state"),
            ReceiverEvent::State {
                sender: first_sender.local_addr().expect("first address"),
                state: first,
            }
        );

        send_state(&ignored_sender, &receiver, state(2, Buttons::B));
        send_state(&first_sender, &receiver, second);

        assert_eq!(
            receiver.next_event().expect("second state"),
            ReceiverEvent::State {
                sender: first_sender.local_addr().expect("first address"),
                state: second,
            }
        );
    }

    #[test]
    fn accept_first_sender_does_not_lock_to_malformed_packet() {
        let malformed_sender = bind_sender();
        let valid_sender = bind_sender();
        let mut config = receiver_config();
        config.accept_first_sender = true;
        let mut receiver = bind_receiver(config);
        let expected = state(1, Buttons::Y);

        malformed_sender
            .send_to(
                &[0; 4],
                receiver.socket.local_addr().expect("receiver address"),
            )
            .expect("malformed packet sends");
        assert!(matches!(
            receiver.next_event(),
            Err(ReceiverError::Parse(_))
        ));

        send_state(&valid_sender, &receiver, expected);
        assert_eq!(
            receiver.next_event().expect("state event"),
            ReceiverEvent::State {
                sender: valid_sender.local_addr().expect("valid sender address"),
                state: expected,
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
    fn emits_timeout_once_then_resumes_on_valid_packet() {
        let mut receiver = bind_receiver(receiver_config());
        let sender = bind_sender();
        let expected = state(1, Buttons::START);

        assert_eq!(
            receiver.next_event().expect("timeout event"),
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
