mod arguments;
mod config;
mod doctor;
use crate::control::{self, Client};
use arguments::{Command, ConfigCommand};
use std::error::Error;

pub(crate) fn run(arguments: Vec<std::ffi::OsString>) -> Result<(), Box<dyn Error>> {
    match arguments::parse(&arguments)? {
        Command::Help => print!("{}", include_str!("help.txt")),
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
            print!("{output}");
        }
        Command::Doctor => doctor::run()?,
    }
    Ok(())
}
