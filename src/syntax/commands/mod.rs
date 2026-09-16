mod catalog;

use std::{path::Path, sync::OnceLock};

const BUILTINS: &[&str] = &[
    "alias",
    "autoload",
    "bg",
    "bindkey",
    "break",
    "builtin",
    "cd",
    "command",
    "compadd",
    "continue",
    "dirs",
    "echo",
    "eval",
    "exec",
    "exit",
    "export",
    "fc",
    "fg",
    "functions",
    "getopts",
    "hash",
    "history",
    "jobs",
    "kill",
    "let",
    "local",
    "popd",
    "print",
    "printf",
    "pushd",
    "pwd",
    "read",
    "rehash",
    "return",
    "set",
    "shift",
    "source",
    "suspend",
    "test",
    "times",
    "trap",
    "true",
    "type",
    "typeset",
    "ulimit",
    "umask",
    "unalias",
    "unset",
    "wait",
    "whence",
    "which",
    "zle",
    "zmodload",
];

#[derive(Debug)]
pub(crate) struct CommandEntry {
    name: String,
    detail: &'static str,
    location: Option<std::path::PathBuf>,
}

impl CommandEntry {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn detail(&self) -> &str {
        self.detail
    }

    pub(crate) fn location(&self) -> Option<&std::path::Path> {
        self.location.as_deref()
    }
}

#[derive(Debug)]
pub(crate) struct CommandCatalog {
    entries: Vec<CommandEntry>,
}

static CATALOG: OnceLock<CommandCatalog> = OnceLock::new();

pub(crate) fn catalog() -> &'static CommandCatalog {
    CATALOG.get_or_init(CommandCatalog::build)
}

pub(crate) fn command_available(command: &str, cwd: &Path) -> bool {
    let command = brush_parser::unquote_str(command);
    if command.is_empty() {
        return false;
    }
    if command.contains('/') {
        let path = Path::new(&command);
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            cwd.join(path)
        };
        return catalog::is_executable(&path);
    }
    catalog().entries.iter().any(|entry| entry.name == command)
}

impl CommandCatalog {
    fn build() -> Self {
        catalog::build()
    }

    pub(crate) fn entries(&self) -> &[CommandEntry] {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{catalog, command_available};

    #[test]
    fn catalog_contains_shell_builtins_and_path_commands() {
        assert!(
            catalog()
                .entries()
                .iter()
                .any(|entry| entry.name() == "echo")
        );
        assert!(command_available("sh", Path::new(".")));
        assert!(!command_available(
            "keel-command-that-does-not-exist",
            Path::new(".")
        ));
    }
}
