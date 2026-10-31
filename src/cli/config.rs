use super::ConfigCommand;
use crate::{
    config::Config,
    control::{self, Client, Command, Health},
};
use std::{
    io::{self, Write},
    path::PathBuf,
    process,
};

pub(super) fn path() -> io::Result<PathBuf> {
    if control::connected() {
        return Ok(Health::parse(&Client::current()?.request(Command::Ping)?)?.config);
    }
    crate::config::path()
}

pub(super) fn run(command: ConfigCommand) -> io::Result<()> {
    match command {
        ConfigCommand::Path => println!("{}", path()?.display()),
        ConfigCommand::Check => {
            let path = path()?;
            Config::read_from(&path)?;
            println!(
                "Configuration valid: {}{}",
                path.display(),
                if path.exists() {
                    ""
                } else {
                    " (using defaults)"
                }
            );
        }
        ConfigCommand::Show => {
            let source = if control::connected() {
                Client::current()?.request(Command::ConfigShow)?
            } else {
                Config::read_from(&path()?)?.effective()
            };
            print!("{source}");
        }
        ConfigCommand::Edit => edit()?,
    }
    Ok(())
}

fn edit() -> io::Result<()> {
    let path = path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(mut file) => file.write_all(include_bytes!("../config/defaults.toml"))?,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    }
    let editor = ["VISUAL", "EDITOR"]
        .iter()
        .find_map(|key| {
            std::env::var(key)
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(|| "vi".into());
    // The editor is a user-supplied shell command; the config path stays a
    // separate positional argument so spaces or shell syntax in it are literal.
    let status = process::Command::new("sh")
        .args(["-c", &format!("exec {editor} \"$1\""), "sopra config edit"])
        .arg(&path)
        .status()?;
    if !status.success() {
        return Err(io::Error::other(format!("editor exited with {status}")));
    }
    Config::read_from(&path)?;
    if control::connected() {
        print!("{}", Client::current()?.request(Command::Refresh)?);
    } else {
        println!("Configuration valid; no active session to refresh.");
    }
    Ok(())
}
