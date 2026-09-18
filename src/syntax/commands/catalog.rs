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
    if !add_exported_commands(&mut commands)
        && let Some(path) = env::var_os("PATH")
    {
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

fn add_exported_commands(commands: &mut BTreeMap<String, CommandEntry>) -> bool {
    let Some(raw) = env::var_os("KEEL_COMMANDS") else {
        return false;
    };
    add_exported_commands_from(&raw.to_string_lossy(), commands)
}

fn add_exported_commands_from(raw: &str, commands: &mut BTreeMap<String, CommandEntry>) -> bool {
    let mut added = false;
    for line in raw.lines() {
        let Some((name, location)) = line.split_once('\t') else {
            continue;
        };
        if name.is_empty() || location.is_empty() {
            continue;
        }
        commands.entry(name.to_string()).or_insert_with(|| {
            added = true;
            CommandEntry {
                name: name.to_string(),
                detail: "path",
                location: Some(location.into()),
            }
        });
    }
    added
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::add_exported_commands_from;
    use crate::syntax::commands::CommandEntry;

    #[test]
    fn exported_commands_keep_existing_precedence_and_locations() {
        let mut commands = BTreeMap::from([(
            "echo".to_string(),
            CommandEntry {
                name: "echo".to_string(),
                detail: "builtin",
                location: None,
            },
        )]);
        assert!(add_exported_commands_from(
            "echo\t/bin/echo\nrg\t/bin/rg\nmalformed",
            &mut commands,
        ));
        assert_eq!(commands["echo"].location(), None);
        assert_eq!(
            commands["rg"].location().and_then(|path| path.to_str()),
            Some("/bin/rg")
        );
    }
}
