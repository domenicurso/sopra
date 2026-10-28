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
    pub(crate) history_file: Option<PathBuf>,
}

impl Args {
    pub(crate) fn parse() -> Result<Self, String> {
        let mut args = Self::defaults();
        let mut arguments = env::args_os().skip(1);
        while let Some(argument) = arguments.next() {
            let argument = argument.to_string_lossy();
            if is_help_argument(&argument) {
                print_usage();
                std::process::exit(0);
            }
            args.read_argument(&argument, &mut arguments)?;
        }
        Ok(args)
    }

    fn defaults() -> Self {
        Self {
            buffer: String::new(),
            cursor: 0,
            arm_completion: false,
            suppress_completion: false,
            completion_enabled: true,
            syntax_highlighting: true,
            cursor_animation: true,
            prompt: "$ ".to_string(),
            rprompt: String::new(),
            cwd: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            history_file: None,
        }
    }

    fn read_argument(
        &mut self,
        argument: &str,
        arguments: &mut impl Iterator<Item = OsString>,
    ) -> Result<(), String> {
        match argument {
            "--buffer" => self.buffer = next_value(arguments, argument)?,
            "--cursor" => {
                self.cursor = next_value(arguments, argument)?
                    .parse()
                    .map_err(|_| "--cursor must be a character index".to_string())?
            }
            "--arm-completion" => self.arm_completion = true,
            "--suppress-completion" => self.suppress_completion = true,
            "--disable-completion" => self.completion_enabled = false,
            "--disable-syntax-highlighting" => self.syntax_highlighting = false,
            "--disable-cursor-animation" => self.cursor_animation = false,
            "--prompt" => self.prompt = next_value(arguments, argument)?,
            "--rprompt" => self.rprompt = next_value(arguments, argument)?,
            "--cwd" => self.cwd = PathBuf::from(next_value(arguments, argument)?),
            "--history-file" => {
                self.history_file = Some(PathBuf::from(next_value(arguments, argument)?))
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
        Ok(())
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
