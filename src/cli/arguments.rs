use std::{ffi::OsString, io};
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Command {
    Help,
    Dev(super::dev::Command),
    Debug(super::debug::Command),
    Version,
    Refresh,
    Restart,
    Config(ConfigCommand),
    Doctor,
    Widgets,
}
#[derive(Debug, PartialEq, Eq)]
pub(super) enum ConfigCommand {
    Edit,
    Check,
    Path,
    Show,
}

pub(super) fn parse(arguments: &[OsString]) -> io::Result<Command> {
    if arguments.first().is_some_and(|name| name == "dev") {
        return super::dev::parse(&arguments[1..]).map(Command::Dev);
    }
    if arguments.first().is_some_and(|name| name == "debug") {
        return super::debug::parse(&arguments[1..]).map(Command::Debug);
    }
    let args = arguments
        .iter()
        .map(|arg| {
            arg.to_str()
                .ok_or_else(|| io::Error::other("command names must be UTF-8"))
        })
        .collect::<io::Result<Vec<_>>>()?;
    Ok(match args.as_slice() {
        [] | ["--help" | "-h" | "help"] | ["config" | "widgets", "--help" | "-h"] => Command::Help,
        ["--version"] => Command::Version,
        ["refresh"] => Command::Refresh,
        ["restart"] => Command::Restart,
        ["doctor"] => Command::Doctor,
        ["widgets", "list"] => Command::Widgets,
        ["config", "edit"] => Command::Config(ConfigCommand::Edit),
        ["config", "check"] => Command::Config(ConfigCommand::Check),
        ["config", "path"] => Command::Config(ConfigCommand::Path),
        ["config", "show"] => Command::Config(ConfigCommand::Show),
        _ => {
            return Err(io::Error::other(
                "unknown command or arguments; run `sopra --help`",
            ));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_commands_reject_extra_arguments_and_private_worker_flags() -> io::Result<()> {
        for (args, expected) in [
            (vec!["refresh"], Command::Refresh),
            (vec!["restart"], Command::Restart),
            (vec!["config", "edit"], Command::Config(ConfigCommand::Edit)),
            (vec!["config", "show"], Command::Config(ConfigCommand::Show)),
            (vec!["widgets", "list"], Command::Widgets),
        ] {
            assert_eq!(
                parse(&args.into_iter().map(Into::into).collect::<Vec<_>>())?,
                expected
            );
        }
        for args in [
            vec!["refresh", "extra"],
            vec!["shell"],
            vec!["shell", "extra"],
            vec!["config"],
            vec!["--serve"],
            vec!["unknown"],
        ] {
            assert!(parse(&args.into_iter().map(Into::into).collect::<Vec<_>>()).is_err());
        }
        Ok(())
    }
}
