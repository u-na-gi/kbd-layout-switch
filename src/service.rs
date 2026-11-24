//! Installing the program as a systemd user service.

use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::process::Command;

use crate::{Options, Result};

const UNIT: &str = "kbd-layout-switch.service";

pub fn install(options: &Options) -> Result<()> {
    let executable = env::current_exe()?;
    let executable = executable
        .to_str()
        .ok_or("the path of this executable is not valid UTF-8")?;
    let command = [
        executable,
        "run",
        "--device",
        &options.device,
        "--connected",
        &options.connected,
        "--disconnected",
        &options.disconnected,
    ]
    .map(quote)
    .join(" ");

    let path = unit_path()?;
    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory)?;
    }
    fs::write(&path, unit(&command))?;
    println!("wrote {}", path.display());

    systemctl(&["daemon-reload"])?;
    systemctl(&["enable", UNIT])?;
    systemctl(&["restart", UNIT])?;
    println!("started {UNIT}");
    Ok(())
}

pub fn uninstall() -> Result<()> {
    let path = unit_path()?;
    if !path.exists() {
        println!("{UNIT} is not installed");
        return Ok(());
    }
    systemctl(&["disable", "--now", UNIT])?;
    match fs::remove_file(&path) {
        Err(error) if error.kind() != ErrorKind::NotFound => return Err(error.into()),
        _ => println!("removed {}", path.display()),
    }
    systemctl(&["daemon-reload"])
}

fn unit(command: &str) -> String {
    format!(
        "[Unit]\n\
         Description=Switch the keyboard layout when a Bluetooth keyboard connects\n\
         PartOf=graphical-session.target\n\
         After=graphical-session.target\n\
         \n\
         [Service]\n\
         ExecStart={command}\n\
         Restart=on-failure\n\
         RestartSec=3\n\
         \n\
         [Install]\n\
         WantedBy=graphical-session.target\n"
    )
}

/// Quote an argument for `ExecStart=`, where `%` and `$` are also special.
fn quote(argument: &str) -> String {
    let mut quoted = String::with_capacity(argument.len() + 2);
    quoted.push('"');
    for character in argument.chars() {
        match character {
            '"' | '\\' => quoted.extend(['\\', character]),
            '%' | '$' => quoted.extend([character, character]),
            _ => quoted.push(character),
        }
    }
    quoted.push('"');
    quoted
}

fn unit_path() -> Result<PathBuf> {
    let config = match env::var_os("XDG_CONFIG_HOME") {
        Some(directory) if !directory.is_empty() => PathBuf::from(directory),
        _ => PathBuf::from(env::var_os("HOME").ok_or("HOME is not set")?).join(".config"),
    };
    Ok(config.join("systemd/user").join(UNIT))
}

fn systemctl(arguments: &[&str]) -> Result<()> {
    let status = Command::new("systemctl")
        .arg("--user")
        .args(arguments)
        .status()
        .map_err(|error| format!("failed to run systemctl: {error}"))?;
    if !status.success() {
        return Err(format!("systemctl --user {} failed", arguments.join(" ")).into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_arguments_for_systemd() {
        assert_eq!(quote("Split65"), r#""Split65""#);
        assert_eq!(quote("My Keyboard"), r#""My Keyboard""#);
        assert_eq!(quote(r#"a"b\c"#), r#""a\"b\\c""#);
        assert_eq!(quote("100% $HOME"), r#""100%% $$HOME""#);
    }
}
