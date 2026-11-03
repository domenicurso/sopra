use crate::control::{self, Client};
use std::{ffi::OsString, io};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Command {
    Help,
    Status,
    Clear(Option<String>),
}

pub(super) fn parse(arguments: &[OsString]) -> io::Result<Command> {
    let words = arguments
        .iter()
        .map(|word| word.to_str())
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| io::Error::other("cache arguments must be UTF-8"))?;
    match words.as_slice() {
        [] | ["--help" | "-h"] => Ok(Command::Help),
        ["status"] => Ok(Command::Status),
        ["clear"] => Ok(Command::Clear(None)),
        ["clear", command] if !command.starts_with('-') => {
            sopra_completion::cache::validate_command(command)?;
            Ok(Command::Clear(Some((*command).into())))
        }
        _ => Err(io::Error::other(
            "invalid cache arguments; run `sopra cache --help`",
        )),
    }
}

pub(super) fn run(command: Command) -> io::Result<()> {
    let request = match command {
        Command::Help => return super::help::print(&super::help_pages::CACHE),
        Command::Status => control::Command::CacheStatus,
        Command::Clear(command) => control::Command::CacheClear(command),
    };
    let report = if control::connected() {
        Client::current()?.request(request).map_err(|error| {
            if error.to_string().contains("unknown worker control request") {
                io::Error::other(format!("{error}; the worker predates cache commands, so run `sopra restart` after rebuilding"))
            } else {
                error
            }
        })?
    } else {
        match request {
            control::Command::CacheStatus => sopra_completion::cache::status()?,
            control::Command::CacheClear(command) => {
                sopra_completion::cache::clear(command.as_deref())?
            }
            _ => unreachable!("cache CLI constructs only cache control requests"),
        }
    };
    super::output::print(&report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_commands_require_an_unambiguous_scope() -> io::Result<()> {
        assert_eq!(parse(&["status".into()])?, Command::Status);
        assert_eq!(parse(&["clear".into()])?, Command::Clear(None));
        assert_eq!(
            parse(&["clear".into(), "git".into()])?,
            Command::Clear(Some("git".into()))
        );
        for words in [
            vec!["status", "git"],
            vec!["clear", "git", "add"],
            vec!["clear", "--wat"],
            vec!["clear", ""],
            vec!["clear", "a\nb"],
        ] {
            assert!(parse(&words.into_iter().map(Into::into).collect::<Vec<_>>()).is_err());
        }
        Ok(())
    }
}
