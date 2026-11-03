use std::{env, ffi::OsString, io, path::PathBuf};

#[derive(Debug, PartialEq, Eq)]
enum Mode {
    Full,
    Root,
    Chunk,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Graph {
    mode: Mode,
    refresh: bool,
    command: String,
    path: Vec<String>,
    cwd: PathBuf,
}

impl Graph {
    pub(super) fn parse(arguments: &[OsString]) -> io::Result<Self> {
        let mut mode = None;
        let mut refresh = false;
        let mut cwd = None;
        let mut words = Vec::new();
        let mut arguments = arguments.iter();
        while let Some(argument) = arguments.next() {
            match argument.to_str() {
                Some("--root" | "--chunk") if mode.is_none() => {
                    mode = Some(if argument == "--root" {
                        Mode::Root
                    } else {
                        Mode::Chunk
                    });
                }
                Some("--fresh") if !refresh => refresh = true,
                Some("--cwd") if cwd.is_none() => {
                    cwd =
                        Some(PathBuf::from(arguments.next().ok_or_else(|| {
                            io::Error::other("--cwd needs a directory")
                        })?));
                }
                Some(word) if !word.starts_with('-') => words.push(word.to_string()),
                _ => {
                    return Err(io::Error::other(
                        "invalid completion graph arguments; run `sopra debug --help`",
                    ));
                }
            }
        }
        let mode = mode.unwrap_or(Mode::Full);
        if words.is_empty() || (mode != Mode::Chunk && words.len() != 1) {
            return Err(io::Error::other(
                "supply a command, and use --chunk for a nested command path",
            ));
        }
        let command = words.remove(0);
        Ok(Self {
            mode,
            refresh,
            command,
            path: words,
            cwd: cwd.map_or_else(env::current_dir, Ok)?,
        })
    }

    pub(super) fn query(&self) -> sopra_completion::inspection::GraphQuery<'_> {
        use sopra_completion::inspection::{GraphMode, GraphQuery};
        GraphQuery {
            command: &self.command,
            path: &self.path,
            cwd: &self.cwd,
            refresh: self.refresh,
            mode: match self.mode {
                Mode::Full => GraphMode::Recursive,
                Mode::Root => GraphMode::Root,
                Mode::Chunk => GraphMode::Chunk,
            },
        }
    }

    pub(super) fn run(&self) -> io::Result<()> {
        let graph =
            sopra_completion::inspection::generate(&self.query()).map_err(io::Error::other)?;
        super::output::print(&graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_arguments_preserve_nested_paths_and_reject_ambiguous_modes() -> io::Result<()> {
        let args = ["--chunk", "npm", "access", "--cwd", "/tmp"].map(Into::into);
        let graph = Graph::parse(&args)?;
        assert_eq!(graph.mode, Mode::Chunk);
        assert_eq!(graph.command, "npm");
        assert_eq!(graph.path, ["access"]);
        assert_eq!(graph.cwd, PathBuf::from("/tmp"));
        for args in [
            vec![],
            vec!["--cwd"],
            vec!["--root", "--chunk", "npm"],
            vec!["npm", "access"],
            vec!["--root", "npm", "access"],
            vec!["--unknown", "npm"],
            vec!["npm", "--cwd", ".", "--cwd", "."],
        ] {
            assert!(Graph::parse(&args.into_iter().map(Into::into).collect::<Vec<_>>()).is_err());
        }
        Ok(())
    }
}
