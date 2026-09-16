use std::{collections::BTreeMap, env, fs, path::Path, sync::OnceLock};

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
}

impl CommandEntry {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn detail(&self) -> &str {
        self.detail
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
        return is_executable(&path);
    }
    catalog().entries.iter().any(|entry| entry.name == command)
}

impl CommandCatalog {
    fn build() -> Self {
        let mut commands = BTreeMap::new();
        for builtin in BUILTINS {
            commands.insert((*builtin).to_string(), "builtin");
        }
        if let Some(path) = env::var_os("PATH") {
            for directory in env::split_paths(&path) {
                add_path_commands(&mut commands, &directory);
            }
        }
        let entries = commands
            .into_iter()
            .map(|(name, detail)| CommandEntry { name, detail })
            .collect();
        Self { entries }
    }

    pub(crate) fn entries(&self) -> &[CommandEntry] {
        &self.entries
    }
}

fn add_path_commands(commands: &mut BTreeMap<String, &'static str>, directory: &Path) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !is_executable(&path) {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        commands.entry(name.to_string()).or_insert("path");
    }
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|metadata| metadata.is_file())
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
