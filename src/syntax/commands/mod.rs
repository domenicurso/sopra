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

    pub(crate) fn visible_for_completion(&self, query: &str) -> bool {
        if self.name.starts_with('.') {
            return query.starts_with('.') || query.starts_with(&self.name);
        }
        if self.detail != "path" {
            return true;
        }
        let resource = self.location.as_deref().is_some_and(is_resource_path);
        let helper = self
            .location
            .as_deref()
            .is_some_and(|path| is_system_helper_name(&self.name, path));
        let first = self.name.chars().next();
        !resource && (!helper || first.is_some_and(|character| query.starts_with(character)))
    }
}

fn is_resource_path(path: &std::path::Path) -> bool {
    path.components()
        .any(|component| component.as_os_str() == "Resources")
        || path
            .extension()
            .is_some_and(|extension| matches!(extension.to_str(), Some("car" | "icns" | "lproj")))
}

fn is_system_helper_name(name: &str, path: &std::path::Path) -> bool {
    let mixed_case =
        name.chars().next().is_some_and(char::is_uppercase) && name.chars().any(char::is_lowercase);
    mixed_case
        && path.components().any(|component| {
            matches!(
                component.as_os_str().to_str(),
                Some("bin" | "sbin" | "usr" | "System" | "Library")
            )
        })
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
    use std::path::{Path, PathBuf};

    use super::{CommandEntry, catalog, command_available};

    #[test]
    fn catalog_contains_shell_builtins_and_path_commands() {
        assert!(
            catalog()
                .entries()
                .iter()
                .any(|entry| entry.name() == "echo")
        );
        assert!(command_available("sh", Path::new(".")));
        assert!(!command_available("missing-command", Path::new(".")));
    }

    #[test]
    fn completion_hides_resource_and_system_helper_entries_until_requested() {
        let resource = CommandEntry {
            name: ".anaconda-navigator-post-link.sh".to_string(),
            detail: "path",
            location: Some(PathBuf::from(
                "/opt/anaconda3/Resources/.anaconda-navigator-post-link.sh",
            )),
        };
        let helper = CommandEntry {
            name: "SystemHelper".to_string(),
            detail: "path",
            location: Some(PathBuf::from("/usr/bin/SystemHelper")),
        };
        assert!(!resource.visible_for_completion(""));
        assert!(resource.visible_for_completion("."));
        assert!(!helper.visible_for_completion(""));
        assert!(helper.visible_for_completion("S"));
    }
}
