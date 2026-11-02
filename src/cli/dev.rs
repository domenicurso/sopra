use std::{ffi::OsString, io, path::PathBuf, process};

use super::{dev_checkout, help, help_pages};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Command {
    Help,
    Build,
    Check,
    Test(Vec<OsString>),
    Shell,
    Link(PathBuf),
    Unlink,
    Path,
}

pub(super) fn parse(arguments: &[OsString]) -> io::Result<Command> {
    let Some((name, rest)) = arguments.split_first() else {
        return Ok(Command::Help);
    };
    Ok(match (name.to_str(), rest) {
        (Some("--help" | "-h" | "help"), []) => Command::Help,
        (Some("build" | "check" | "test" | "shell" | "link" | "unlink" | "path"), [flag])
            if flag == "--help" || flag == "-h" =>
        {
            Command::Help
        }
        (Some("build"), []) => Command::Build,
        (Some("check"), []) => Command::Check,
        (Some("test"), rest) => Command::Test(rest.to_vec()),
        (Some("shell"), []) => Command::Shell,
        (Some("link"), [path]) => Command::Link(path.into()),
        (Some("unlink"), []) => Command::Unlink,
        (Some("path"), []) => Command::Path,
        _ => {
            return Err(io::Error::other(
                "unknown developer command or arguments; run `sopra dev --help`",
            ));
        }
    })
}

pub(super) fn run(command: Command) -> io::Result<()> {
    match command {
        Command::Help => help::print(&help_pages::DEV),
        Command::Link(path) => {
            println!(
                "Linked developer checkout: {}",
                dev_checkout::link(&path)?.display()
            );
            Ok(())
        }
        Command::Unlink => {
            dev_checkout::unlink()?;
            println!("Developer checkout link removed.");
            Ok(())
        }
        Command::Path => {
            println!("{}", dev_checkout::current()?.display());
            Ok(())
        }
        command => run_in_checkout(command),
    }
}

fn run_in_checkout(command: Command) -> io::Result<()> {
    let checkout = dev_checkout::current()?;
    let mut steps = checkout_commands(command, &checkout)?;
    for step in &mut steps {
        let status = step.current_dir(&checkout).status()?;
        if !status.success() {
            return Err(io::Error::other(format!(
                "{} exited with {status}",
                step.get_program().to_string_lossy()
            )));
        }
    }
    Ok(())
}

fn checkout_commands(
    command: Command,
    checkout: &std::path::Path,
) -> io::Result<Vec<process::Command>> {
    Ok(match command {
        Command::Build => vec![process::Command::new(checkout.join("scripts/build.sh"))],
        Command::Shell => vec![process::Command::new(checkout.join("scripts/start.sh"))],
        Command::Test(arguments) => {
            let mut test = process::Command::new("cargo");
            test.args(["test", "--workspace", "--locked"])
                .args(arguments);
            vec![test]
        }
        Command::Check => {
            let mut format = process::Command::new("cargo");
            format.args(["fmt", "--all", "--", "--check"]);
            let mut lint = process::Command::new("cargo");
            lint.args([
                "clippy",
                "--workspace",
                "--all-targets",
                "--locked",
                "--",
                "-D",
                "warnings",
            ]);
            vec![format, lint]
        }
        _ => {
            return Err(io::Error::other(
                "developer command does not run in a checkout",
            ));
        }
    })
}
