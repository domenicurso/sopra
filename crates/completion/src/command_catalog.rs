use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use super::command_discovery::{add_exported_commands, add_named_commands, add_path_commands};

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

#[derive(Debug, Clone)]
pub(crate) struct CommandEntry {
    pub(crate) name: String,
    pub(crate) detail: &'static str,
    pub(crate) location: Option<PathBuf>,
}

impl CommandEntry {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn detail(&self) -> &str {
        self.detail
    }

    pub(crate) fn location(&self) -> Option<&Path> {
        self.location.as_deref()
    }

    pub(crate) fn visible(&self, query: &str) -> bool {
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
        !resource
            && (!helper
                || self
                    .name
                    .chars()
                    .next()
                    .is_some_and(|character| query.starts_with(character)))
    }
}

static CATALOG: OnceLock<Vec<CommandEntry>> = OnceLock::new();

pub(crate) fn catalog() -> &'static [CommandEntry] {
    CATALOG.get_or_init(build).as_slice()
}

fn build() -> Vec<CommandEntry> {
    let mut commands = BTreeMap::new();
    for name in BUILTINS {
        commands.insert(
            (*name).to_string(),
            CommandEntry {
                name: (*name).to_string(),
                detail: "builtin",
                location: None,
            },
        );
    }
    if !add_exported_commands(&mut commands)
        && let Some(path) = std::env::var_os("PATH")
    {
        for directory in std::env::split_paths(&path) {
            add_path_commands(&mut commands, &directory);
        }
    }
    add_named_commands(&mut commands, "SOPRA_ALIASES", "alias");
    add_named_commands(&mut commands, "SOPRA_FUNCTIONS", "function");
    commands.into_values().collect()
}

fn is_resource_path(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str() == "Resources")
        || path
            .extension()
            .is_some_and(|extension| matches!(extension.to_str(), Some("car" | "icns" | "lproj")))
}

fn is_system_helper_name(name: &str, path: &Path) -> bool {
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

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{CommandEntry, is_resource_path, is_system_helper_name};

    #[test]
    fn command_completion_hides_system_helpers_until_requested() {
        let helper = CommandEntry {
            name: "SystemHelper".to_string(),
            detail: "path",
            location: Some(PathBuf::from("/usr/bin/SystemHelper")),
        };
        assert!(!helper.visible(""));
        assert!(helper.visible("S"));
        assert!(is_system_helper_name(
            "SystemHelper",
            Path::new("/usr/bin/SystemHelper")
        ));
        assert!(is_resource_path(Path::new("/opt/App/Resources/tool")));
    }
}
