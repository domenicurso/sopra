use std::{env, ffi::OsString, path::PathBuf};

#[derive(Debug)]
pub(crate) struct Args {
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
                "--buffer" => {
                    buffer = next_value(&mut arguments, "--buffer")?;
                }
                "--cursor" => {
                    cursor = next_value(&mut arguments, "--cursor")?
                        .parse()
                        .map_err(|_| "--cursor must be a character index".to_string())?;
                }
                "--arm-completion" => {
                    arm_completion = true;
                }
                "--suppress-completion" => {
                    suppress_completion = true;
                }
                "--disable-completion" => {
                    completion_enabled = false;
                }
                "--disable-syntax-highlighting" => {
                    syntax_highlighting = false;
                }
                "--disable-cursor-animation" => {
                    cursor_animation = false;
                }
                "--prompt" => {
                    prompt = next_value(&mut arguments, "--prompt")?;
                }
                "--rprompt" => {
                    rprompt = next_value(&mut arguments, "--rprompt")?;
                }
                "--cwd" => {
                    cwd = PathBuf::from(next_value(&mut arguments, "--cwd")?);
                }
                argument if is_help_argument(argument) => help = true,
                unknown => return Err(format!("unknown argument: {unknown}")),
            }
        }

        if help {
            print_usage();
            std::process::exit(0);
        }

        Ok(Self {
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
        "sopra\n\nSopra is configured in ~/.config/sopra/config.toml, or in $XDG_CONFIG_HOME/sopra/config.toml when XDG_CONFIG_HOME is set."
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
