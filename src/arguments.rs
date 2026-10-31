use std::{env, ffi::OsString, path::PathBuf};

#[derive(Debug)]
pub(crate) struct Args {
    pub(crate) buffer: String,
    pub(crate) cursor: usize,
    pub(crate) arm_completion: bool,
    pub(crate) suppress_completion: bool,
    pub(crate) secondary: bool,
    pub(crate) cwd: PathBuf,
    pub(crate) history_file: Option<PathBuf>,
}

impl Args {
    pub(crate) fn parse() -> Result<Self, String> {
        Self::from_args(env::args_os().skip(1))
    }

    pub(crate) fn from_args(mut arguments: impl Iterator<Item = OsString>) -> Result<Self, String> {
        let mut args = Self::defaults();
        while let Some(argument) = arguments.next() {
            let argument = argument.to_string_lossy();
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
            secondary: false,
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
            "--secondary" => self.secondary = true,
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
