use std::{env, ffi::OsString, io, path::PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Candidates {
    line: String,
    cursor: usize,
    cwd: PathBuf,
}

impl Candidates {
    pub(super) fn parse(arguments: &[OsString]) -> io::Result<Self> {
        let mut cursor = None;
        let mut cwd = None;
        let mut line = None;
        let mut words = arguments.iter();
        while let Some(word) = words.next() {
            match word.to_str() {
                Some("--cursor") if cursor.is_none() => {
                    cursor = Some(
                        words
                            .next()
                            .and_then(|word| word.to_str())
                            .and_then(|word| word.parse().ok())
                            .ok_or_else(|| {
                                io::Error::other("--cursor requires a character offset")
                            })?,
                    );
                }
                Some("--cwd") if cwd.is_none() => {
                    cwd =
                        Some(PathBuf::from(words.next().ok_or_else(|| {
                            io::Error::other("--cwd requires a directory")
                        })?));
                }
                Some(value) if line.is_none() => line = Some(value.to_string()),
                _ => {
                    return Err(io::Error::other(
                        "supply one shell line; run `sopra debug candidates --help`",
                    ));
                }
            }
        }
        let line = line.ok_or_else(|| io::Error::other("supply a shell line"))?;
        let cursor = cursor.unwrap_or_else(|| line.chars().count());
        if cursor > line.chars().count() {
            return Err(io::Error::other("cursor exceeds the input character count"));
        }
        Ok(Self {
            line,
            cursor,
            cwd: cwd.map_or_else(env::current_dir, Ok)?,
        })
    }

    pub(super) fn run(&self) -> io::Result<()> {
        super::output::print(
            &sopra_completion::inspection::candidates(&self.line, self.cursor, &self.cwd)
                .map_err(io::Error::other)?,
        )
    }
}
