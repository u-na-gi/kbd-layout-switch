//! Switch the GNOME keyboard layout when a Bluetooth keyboard connects.

mod bluez;
mod service;
mod sources;

use std::process::ExitCode;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use clap::{Args, Parser, Subcommand};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, Error>;

/// Bluetooth events arrive in bursts; wait for them to settle before acting.
const DEBOUNCE: Duration = Duration::from_millis(500);

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Watch Bluetooth connections and switch the layout (runs until killed)
    Run(Options),
    /// Install and start a systemd user service that runs `run` with these options
    Install(Options),
    /// Stop and remove the systemd user service
    Uninstall,
}

#[derive(Args, Clone)]
struct Options {
    /// Name prefix of the Bluetooth keyboard, as shown by `bluetoothctl devices`
    #[arg(long, value_name = "PREFIX")]
    device: String,

    /// xkb layout while the keyboard is connected (`layout` or `layout+variant`)
    #[arg(long, value_name = "LAYOUT", default_value = "us")]
    connected: String,

    /// xkb layout while the keyboard is not connected
    #[arg(long, value_name = "LAYOUT", default_value = "jp")]
    disconnected: String,
}

fn main() -> ExitCode {
    let result = match Cli::parse().command {
        Command::Run(options) => run(&options),
        Command::Install(options) => service::install(&options),
        Command::Uninstall => service::uninstall(),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(options: &Options) -> Result<()> {
    let (events, wakeups) = mpsc::channel();
    let watcher = thread::spawn(move || bluez::watch(events));
    let connection = bluez::connect()?;

    let mut last = None;

    // The watcher sends one event as soon as it is subscribed, so the layout
    // is also brought in line with the current state at startup.
    while wakeups.recv().is_ok() {
        loop {
            match wakeups.recv_timeout(DEBOUNCE) {
                Ok(()) => continue,
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        let connected = bluez::is_connected(&connection, &options.device)?;
        if last != Some(connected) {
            let state = if connected {
                "connected"
            } else {
                "not connected"
            };
            println!("{} is {state}", options.device);
            last = Some(connected);
        }
        let layout = if connected {
            &options.connected
        } else {
            &options.disconnected
        };
        if sources::switch_to(layout)? {
            println!("switched layout to {layout}");
        }
    }

    // The channel only closes when the watcher has stopped.
    match watcher.join() {
        Ok(Err(error)) => Err(error),
        Ok(Ok(())) => Err("lost the connection to the system bus".into()),
        Err(_) => Err("the Bluetooth watcher panicked".into()),
    }
}
