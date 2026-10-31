mod arguments;
mod config;
mod doctor;
mod help;
mod output;
mod toml;
mod widgets;
use crate::control::{self, Client};
use arguments::{Command, ConfigCommand};
use std::error::Error;

pub(crate) fn run(arguments: Vec<std::ffi::OsString>) -> Result<(), Box<dyn Error>> {
    match arguments::parse(&arguments)? {
        Command::Help => help::print()?,
        Command::Version => println!("sopra {}", env!("CARGO_PKG_VERSION")),
        Command::Refresh => print!("{}", Client::current()?.request(control::Command::Refresh)?),
        Command::Restart => {
            Client::current()?.restart()?;
            println!("Sopra restarted.");
        }
        Command::Config(command) => config::run(command)?,
        Command::Widgets => {
            let output = if control::connected() {
                Client::current()?.request(control::Command::WidgetsList)?
            } else {
                let config = crate::config::Config::read()?;
                crate::prompt::widgets::inspection::list(&config.widgets, None)?
            };
            output::print(&widgets::render(&output, &output::Output::detect()))?;
        }
        Command::Doctor => doctor::run()?,
        Command::Shell => {
            let code = crate::host::run()?;
            if code != 0 {
                std::process::exit(code.min(255) as i32);
            }
        }
    }
    Ok(())
}
