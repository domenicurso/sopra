use std::{collections::BTreeMap, env, fs, path::Path};

use super::{BUILTINS, CommandCatalog, CommandEntry};

pub(super) fn build() -> CommandCatalog {
    let mut commands = BTreeMap::new();
    for builtin in BUILTINS {
        commands.insert(
            (*builtin).to_string(),
            CommandEntry {
                name: (*builtin).to_string(),
                detail: "builtin",
                location: None,
            },
        );
    }
    if let Some(path) = env::var_os("PATH") {
        for directory in env::split_paths(&path) {
            add_path_commands(&mut commands, &directory);
        }
    }
    add_named_commands(&mut commands, "KEEL_ALIASES", "alias");
    add_named_commands(&mut commands, "KEEL_FUNCTIONS", "function");
    CommandCatalog {
        entries: commands.into_values().collect(),
    }
}

fn add_path_commands(commands: &mut BTreeMap<String, CommandEntry>, directory: &Path) {
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
        commands.entry(name.to_string()).or_insert(CommandEntry {
            name: name.to_string(),
            detail: "path",
            location: Some(path),
        });
    }
}

fn add_named_commands(
    commands: &mut BTreeMap<String, CommandEntry>,
    variable: &str,
    detail: &'static str,
) {
    let Some(names) = env::var_os(variable) else {
        return;
    };
    for name in names.to_string_lossy().split_whitespace() {
        if name.is_empty() || (detail == "function" && name.starts_with('_')) {
            continue;
        }
        commands.insert(
            name.to_string(),
            CommandEntry {
                name: name.to_string(),
                detail,
                location: None,
            },
        );
    }
}

#[cfg(unix)]
pub(super) fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
pub(super) fn is_executable(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|metadata| metadata.is_file())
}
