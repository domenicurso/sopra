use std::{env, io, process::ExitCode};

use keel_core::{PromptConfig, PromptToken, RuntimeOutcome, SessionConfig};
use keel_shell::ShellContextCollector;
use keel_terminal::{CrosstermTerminal, run_command_read_session};

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliOptions {
    prompt: String,
    initial_buffer: String,
    initial_cursor: Option<usize>,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            prompt: "keel> ".to_string(),
            initial_buffer: String::new(),
            initial_cursor: None,
        }
    }
}

impl CliOptions {
    fn parse<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = String>,
    {
        let mut options = Self::default();
        let mut args = args.into_iter();

        while let Some(argument) = args.next() {
            match argument.as_str() {
                "--prompt" => {
                    options.prompt = args
                        .next()
                        .ok_or_else(|| "missing value for --prompt".to_string())?;
                }
                "--buffer" => {
                    options.initial_buffer = args
                        .next()
                        .ok_or_else(|| "missing value for --buffer".to_string())?;
                }
                "--cursor" => {
                    let raw = args
                        .next()
                        .ok_or_else(|| "missing value for --cursor".to_string())?;
                    options.initial_cursor = Some(
                        raw.parse::<usize>()
                            .map_err(|_| "invalid value for --cursor".to_string())?,
                    );
                }
                "--help" | "-h" => {
                    return Err(Self::usage());
                }
                unknown => {
                    return Err(format!("unknown argument: {unknown}\n\n{}", Self::usage()));
                }
            }
        }

        Ok(options)
    }

    fn usage() -> String {
        "usage: keel-shell [--prompt <text>] [--buffer <text>] [--cursor <index>]".to_string()
    }
}

fn main() -> ExitCode {
    match try_main() {
        Ok(RuntimeOutcome::Accepted(command)) => {
            println!("{command}");
            ExitCode::SUCCESS
        }
        Ok(RuntimeOutcome::Cancelled) => ExitCode::from(130),
        Err(error) => {
            eprintln!("keel-shell: {error}");
            ExitCode::FAILURE
        }
    }
}

fn try_main() -> Result<RuntimeOutcome, Box<dyn std::error::Error>> {
    let options = CliOptions::parse(env::args().skip(1))
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;

    let mut terminal = CrosstermTerminal::enter_with_stderr()?;
    let initial_cursor = options
        .initial_cursor
        .unwrap_or_else(|| options.initial_buffer.chars().count());
    let outcome = run_command_read_session(
        &mut terminal,
        SessionConfig {
            prompt: PromptConfig {
                active_left: vec![PromptToken::Literal(options.prompt.clone())],
                active_right: Vec::new(),
                transient_left: vec![PromptToken::Literal("> ".to_string())],
            },
            initial_buffer: options.initial_buffer,
            initial_cursor,
            shell: ShellContextCollector::capture(),
            ..SessionConfig::default()
        },
    )?;

    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::CliOptions;

    #[test]
    fn parses_prompt_and_buffer() {
        let options = CliOptions::parse([
            "--prompt".to_string(),
            "demo> ".to_string(),
            "--buffer".to_string(),
            "echo hi".to_string(),
            "--cursor".to_string(),
            "3".to_string(),
        ])
        .unwrap();

        assert_eq!(options.prompt, "demo> ");
        assert_eq!(options.initial_buffer, "echo hi");
        assert_eq!(options.initial_cursor, Some(3));
    }

    #[test]
    fn rejects_unknown_flags() {
        let error = CliOptions::parse(["--wat".to_string()]).unwrap_err();
        assert!(error.contains("unknown argument"));
    }
}
