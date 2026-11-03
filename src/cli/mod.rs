mod arguments;
mod config;
mod debug;
mod debug_candidates;
mod debug_graph;
mod debug_graph_commands;
mod debug_syntax;
mod dev;
mod dev_checkout;
mod doctor;
mod help;
mod help_pages;
mod help_topics;
mod output;
mod toml;
mod widgets;
use crate::control::{self, Client};
use arguments::{Command, ConfigCommand};
use std::error::Error;

pub(crate) fn run(arguments: Vec<std::ffi::OsString>) -> Result<(), Box<dyn Error>> {
    if help_topics::print_requested(&arguments)? {
        return Ok(());
    }
    match arguments::parse(&arguments)? {
        Command::Help => help::print(&help_pages::ROOT)?,
        Command::Dev(command) => dev::run(command)?,
        Command::Debug(command) => debug::run(command)?,
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
    }
    Ok(())
}
