use std::{env, ffi::OsString, path::PathBuf};

#[derive(Debug)]
pub(crate) struct Args {
    pub(crate) settings: bool,
    pub(crate) buffer: String,
    pub(crate) cursor: usize,
    pub(crate) arm_completion: bool,
    pub(crate) suppress_completion: bool,
    pub(crate) completion_enabled: bool,
    pub(crate) syntax_highlighting: bool,
    pub(crate) cursor_animation: bool,
    pub(crate) prompt: String,
    pub(crate) rprompt: String,
    pub(crate) cwd: PathBuf,
}

impl Args {
    pub(crate) fn parse() -> Result<Self, String> {
        let mut buffer = String::new();
        let mut settings = false;
        let mut editor_arguments = false;
        let mut cursor = 0;
        let mut arm_completion = false;
        let mut suppress_completion = false;
        let mut completion_enabled = true;
        let mut syntax_highlighting = true;
        let mut cursor_animation = true;
        let mut prompt = "$ ".to_string();
        let mut rprompt = String::new();
        let mut help = false;
        let mut cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut arguments = env::args_os().skip(1);

        while let Some(argument) = arguments.next() {
            match argument.to_string_lossy().as_ref() {
                "--settings" | "settings" => settings = true,
                "--buffer" => {
                    editor_arguments = true;
                    buffer = next_value(&mut arguments, "--buffer")?;
                }
                "--cursor" => {
                    editor_arguments = true;
                    cursor = next_value(&mut arguments, "--cursor")?
                        .parse()
                        .map_err(|_| "--cursor must be a character index".to_string())?;
                }
                "--arm-completion" => {
                    editor_arguments = true;
                    arm_completion = true;
                }
                "--suppress-completion" => {
                    editor_arguments = true;
                    suppress_completion = true;
                }
                "--disable-completion" => {
                    editor_arguments = true;
                    completion_enabled = false;
                }
                "--disable-syntax-highlighting" => {
                    editor_arguments = true;
                    syntax_highlighting = false;
                }
                "--disable-cursor-animation" => {
                    editor_arguments = true;
                    cursor_animation = false;
                }
                "--prompt" => {
                    editor_arguments = true;
                    prompt = next_value(&mut arguments, "--prompt")?;
                }
                "--rprompt" => {
                    editor_arguments = true;
                    rprompt = next_value(&mut arguments, "--rprompt")?;
                }
                "--cwd" => {
                    editor_arguments = true;
                    cwd = PathBuf::from(next_value(&mut arguments, "--cwd")?);
                }
                argument if is_help_argument(argument) => help = true,
                unknown => return Err(format!("unknown argument: {unknown}")),
            }
        }

        if settings && editor_arguments {
            return Err("--settings cannot be combined with editor arguments".to_string());
        }
        if help {
            if settings {
                print_settings_usage();
            } else {
                print_usage();
            }
            std::process::exit(0);
        }

        Ok(Self {
            settings,
            buffer,
            cursor,
            arm_completion,
            suppress_completion,
            completion_enabled,
            syntax_highlighting,
            cursor_animation,
            prompt,
            rprompt,
            cwd,
        })
    }
}

fn next_value(
    arguments: &mut impl Iterator<Item = OsString>,
    flag: &str,
) -> Result<String, String> {
    arguments
        .next()
        .map(|value| value.to_string_lossy().into_owned())
        .ok_or_else(|| format!("{flag} needs a value"))
}

fn print_usage() {
    println!(
        "sopra\n\nControls the Sopra shell integration.\n\nUsage:\n  sopra settings\n\nCommands:\n  settings    Open the Sopra settings overlay.\n\nOptions:\n  -h, --help  Print help information."
    );
}

fn print_settings_usage() {
    println!(
        "sopra settings\n\nOpen the Sopra settings overlay.\n\nUsage:\n  sopra settings\n\nOptions:\n  -h, --help  Print help information."
    );
}

fn is_help_argument(argument: &str) -> bool {
    matches!(argument, "--help" | "-h" | "help")
}

#[cfg(test)]
mod tests {
    use super::is_help_argument;

    #[test]
    fn accepts_all_public_help_spellings() {
        for argument in ["--help", "-h", "help"] {
            assert!(is_help_argument(argument));
        }
        assert!(!is_help_argument("settings"));
    }
}
