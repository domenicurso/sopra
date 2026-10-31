use crate::{
    config::Config,
    control::{Client, Command, Health},
};
use std::{io, process};

pub(super) fn run() -> io::Result<()> {
    let mut healthy = true;
    let health = Client::current()
        .and_then(|client| client.request(Command::Ping))
        .and_then(|source| Health::parse(&source));
    match &health {
        Ok(health) => {
            println!(
                "Worker: connected (PID {}, {})",
                health.pid,
                if health.busy { "editing" } else { "ready" }
            );
            println!("Shell integration: connected to Zsh PID {}", health.shell);
        }
        Err(error) => {
            println!("Worker / shell integration: {error}");
            healthy = false;
        }
    }
    let path = health
        .as_ref()
        .map(|health| health.config.clone())
        .map_or_else(|_| crate::config::path(), Ok);
    healthy &= configuration(path);
    healthy &= compiler();
    if healthy {
        Ok(())
    } else {
        Err(io::Error::other(
            "diagnostics found problems; see the checks above",
        ))
    }
}

fn compiler() -> bool {
    match process::Command::new("rustc").arg("--version").output() {
        Ok(output) if output.status.success() => {
            println!(
                "Rust compiler: {}",
                String::from_utf8_lossy(&output.stdout).trim()
            );
            true
        }
        _ => {
            println!("Rust compiler: unavailable; custom Rust widgets cannot compile");
            false
        }
    }
}

fn configuration(path: io::Result<std::path::PathBuf>) -> bool {
    match path.and_then(|path| Config::read_from(&path).map(|config| (path, config))) {
        Ok((path, config)) => {
            println!(
                "Configuration: valid ({}){}",
                path.display(),
                if path.exists() {
                    ""
                } else {
                    "; using defaults"
                }
            );
            let count = match crate::prompt::widgets::catalog::read(&config.widgets) {
                Ok(widgets) => widgets.len(),
                Err(error) => {
                    println!("Widgets: {error}");
                    return false;
                }
            };
            println!("Widgets: {count} discovered");
            true
        }
        Err(error) => {
            println!("Configuration: {error}");
            false
        }
    }
}
