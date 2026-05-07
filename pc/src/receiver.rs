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
                    if !self.accept_sender(sender) {
                        continue;
                    }

                    let state = protocol::parse_controller_state(&buffer[..len])?;

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
