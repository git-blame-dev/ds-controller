mod backend;
mod mapping;
mod protocol;
mod receiver;

use std::env;
use std::net::SocketAddr;
use std::process::ExitCode;
use std::time::Duration;

use backend::create_backend;
use mapping::map_ds_to_xbox;
use receiver::{Receiver, ReceiverConfig, ReceiverEvent};

const DEFAULT_BIND_ADDR: &str = "0.0.0.0:26760";
const DEFAULT_TIMEOUT_MS: u64 = 150;
#[cfg(windows)]
const VIGEM_SETUP_URL: &str = "https://docs.nefarius.at/projects/ViGEm/How-to-Install/";

#[derive(Debug)]
struct Args {
    bind_addr: SocketAddr,
    timeout: Duration,
    accept_first_sender: bool,
    sender: Option<SocketAddr>,
    print_packets: bool,
    no_vigem: bool,
}

impl Args {
    fn parse() -> Result<Self, String> {
        Self::parse_from(env::args().skip(1))
    }

    fn parse_from<I, S>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut bind_addr = DEFAULT_BIND_ADDR
            .parse::<SocketAddr>()
            .map_err(|error| format!("invalid default bind address: {error}"))?;
        let mut timeout = Duration::from_millis(DEFAULT_TIMEOUT_MS);
        let mut accept_first_sender = false;
        let mut sender = None;
        let mut print_packets = false;
        let mut no_vigem = false;

        let mut args = args.into_iter().map(Into::into);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--bind" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "--bind requires <addr:port>".to_owned())?;
                    bind_addr = value
                        .parse::<SocketAddr>()
                        .map_err(|error| format!("invalid --bind value '{value}': {error}"))?;
                }
                "--timeout-ms" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "--timeout-ms requires <ms>".to_owned())?;
                    let millis = value.parse::<u64>().map_err(|error| {
                        format!("invalid --timeout-ms value '{value}': {error}")
                    })?;
                    timeout = Duration::from_millis(millis);
                }
                "--sender" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "--sender requires <addr:port>".to_owned())?;
                    sender =
                        Some(value.parse::<SocketAddr>().map_err(|error| {
                            format!("invalid --sender value '{value}': {error}")
                        })?);
                }
                "--accept-first-sender" => accept_first_sender = true,
                "--print-packets" => print_packets = true,
                "--no-vigem" => no_vigem = true,
                "--help" | "-h" => return Err(Self::usage()),
                _ => return Err(format!("unknown argument '{arg}'\n\n{}", Self::usage())),
            }
        }

        if sender.is_some() && accept_first_sender {
            return Err("use either --sender or --accept-first-sender, not both".to_owned());
        }

        Ok(Self {
            bind_addr,
            timeout,
            accept_first_sender,
            sender,
            print_packets,
            no_vigem,
        })
    }

    fn usage() -> String {
        format!(
            "Usage: ds-controller-pc [OPTIONS]\n\n\
             Options:\n\
               --bind <addr:port>       UDP listen address [default: {DEFAULT_BIND_ADDR}]\n\
               --timeout-ms <ms>        Release-all timeout [default: {DEFAULT_TIMEOUT_MS}]\n\
               --sender <addr:port>     Accept packets only from this sender\n\
               --accept-first-sender    Lock to the first valid packet sender\n\
               --print-packets          Print every accepted packet\n\
               --no-vigem               Run network/protocol receiver without controller output\n\
               -h, --help               Print help"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_args() {
        let args = Args::parse_from([] as [&str; 0]).expect("default args");

        assert_eq!(args.bind_addr, "0.0.0.0:26760".parse().unwrap());
        assert_eq!(args.timeout, Duration::from_millis(150));
        assert!(!args.accept_first_sender);
        assert_eq!(args.sender, None);
        assert!(!args.print_packets);
        assert!(!args.no_vigem);
    }

    #[test]
    fn parses_all_options() {
        let args = Args::parse_from([
            "--bind",
            "127.0.0.1:3000",
            "--timeout-ms",
            "250",
            "--sender",
            "127.0.0.1:4000",
            "--print-packets",
            "--no-vigem",
        ])
        .expect("valid args");

        assert_eq!(args.bind_addr, "127.0.0.1:3000".parse().unwrap());
        assert_eq!(args.timeout, Duration::from_millis(250));
        assert_eq!(args.sender, Some("127.0.0.1:4000".parse().unwrap()));
        assert!(args.print_packets);
        assert!(args.no_vigem);
    }

    #[test]
    fn parses_accept_first_sender() {
        let args = Args::parse_from(["--accept-first-sender"]).expect("valid args");

        assert!(args.accept_first_sender);
    }

    #[test]
    fn rejects_sender_with_accept_first_sender() {
        let error = Args::parse_from(["--sender", "127.0.0.1:4000", "--accept-first-sender"])
            .expect_err("conflicting args");

        assert!(error.contains("use either --sender or --accept-first-sender"));
    }

    #[test]
    fn rejects_missing_option_value() {
        let error = Args::parse_from(["--bind"]).expect_err("missing value");

        assert!(error.contains("--bind requires"));
    }

    #[test]
    fn rejects_invalid_bind_address() {
        let error = Args::parse_from(["--bind", "not-an-address"]).expect_err("invalid bind");

        assert!(error.contains("invalid --bind value"));
    }

    #[test]
    fn rejects_invalid_timeout() {
        let error = Args::parse_from(["--timeout-ms", "nope"]).expect_err("invalid timeout");

        assert!(error.contains("invalid --timeout-ms value"));
    }

    #[test]
    fn rejects_unknown_arg() {
        let error = Args::parse_from(["--bogus"]).expect_err("unknown arg");

        assert!(error.contains("unknown argument"));
    }
}

fn main() -> ExitCode {
    let args = match Args::parse() {
        Ok(args) => args,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };

    println!("listening on {}", args.bind_addr);
    println!("timeout: {} ms", args.timeout.as_millis());
    println!(
        "mode: {}",
        if args.no_vigem {
            "no-vigem protocol receiver"
        } else {
            "ViGEm Xbox 360 output"
        }
    );

    let config = ReceiverConfig {
        bind_addr: args.bind_addr,
        timeout: args.timeout,
        fixed_sender: args.sender,
        accept_first_sender: args.accept_first_sender,
    };

    let mut receiver = match Receiver::bind(config) {
        Ok(receiver) => receiver,
        Err(error) => {
            eprintln!("failed to bind UDP receiver: {error}");
            return ExitCode::FAILURE;
        }
    };

    let mut backend = match create_backend(args.no_vigem) {
        Ok(backend) => backend,
        Err(error) => {
            eprintln!("failed to initialize controller backend: {error}");
            offer_vigem_setup_help(args.no_vigem);
            return ExitCode::FAILURE;
        }
    };

    loop {
        match receiver.next_event() {
            Ok(ReceiverEvent::State { sender, state }) => {
                let output = map_ds_to_xbox(state);
                if let Err(error) = backend.update(output) {
                    eprintln!("controller update failed: {error}");
                }

                if args.print_packets {
                    println!(
                        "{sender} seq={} ds={} xbox={}",
                        state.sequence, state.buttons, output.buttons
                    );
                }
            }
            Ok(ReceiverEvent::Timeout) => {
                if let Err(error) = backend.neutral() {
                    eprintln!("neutral controller update failed: {error}");
                }

                println!("receiver timeout: release all inputs");
            }
            Err(error) => {
                eprintln!("receiver error: {error}");
            }
        }
    }
}

#[cfg(windows)]
fn offer_vigem_setup_help(no_vigem: bool) {
    if no_vigem {
        return;
    }

    eprintln!();
    eprintln!("ViGEmBus is required to create the virtual Xbox 360 controller.");
    eprintln!(
        "Install the Nefarius Virtual Gamepad Emulation Bus Driver, then run this app again."
    );
    eprintln!("Setup instructions: {VIGEM_SETUP_URL}");
    eprintln!("Note: ViGEmBus is a retired third-party kernel driver. Review the publisher before installing.");
}

#[cfg(not(windows))]
fn offer_vigem_setup_help(_no_vigem: bool) {}
