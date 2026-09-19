use std::{
    collections::{BTreeMap, btree_map::Entry},
    env, fs,
    path::{Path, PathBuf},
};

use super::command_catalog::{CommandEntry, catalog};

pub(crate) fn resolve(program: &str, cwd: &Path) -> Option<(String, Option<PathBuf>)> {
    if program.contains('/') {
        let path = Path::new(program);
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            cwd.join(path)
        };
        return is_executable(&path).then(|| (path.display().to_string(), Some(path)));
    }
    let entry = catalog().iter().find(|entry| entry.name == program)?;
    if matches!(entry.detail, "alias" | "function") {
        return None;
    }
    let key = entry.location().map_or_else(
        || format!("builtin:{program}"),
        |path| path.display().to_string(),
    );
    Some((key, entry.location().map(ToOwned::to_owned)))
}

pub fn command_available(command: &str, cwd: &Path) -> bool {
    if command.contains('/') {
        let path = Path::new(command);
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            cwd.join(path)
        };
        return is_executable(&path);
    }
    catalog().iter().any(|entry| entry.name == command)
}

pub(crate) fn add_exported_commands(commands: &mut BTreeMap<String, CommandEntry>) -> bool {
    let Some(raw) = env::var_os("SOPRA_COMMANDS") else {
        return false;
    };
    let mut added = false;
    for line in raw.to_string_lossy().lines() {
        let Some((name, location)) = line.split_once('\t') else {
            continue;
        };
        if name.is_empty() || location.is_empty() {
            continue;
        }
        if let Entry::Vacant(slot) = commands.entry(name.to_string()) {
            slot.insert(CommandEntry {
                name: name.to_string(),
                detail: "path",
                location: Some(PathBuf::from(location)),
            });
            added = true;
        }
    }
    added
}

pub(crate) fn add_path_commands(commands: &mut BTreeMap<String, CommandEntry>, directory: &Path) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if is_executable(&path) {
            commands.entry(name.to_string()).or_insert(CommandEntry {
                name: name.to_string(),
                detail: "path",
                location: Some(path),
            });
        }
    }
}

pub(crate) fn add_named_commands(
    commands: &mut BTreeMap<String, CommandEntry>,
    variable: &str,
    detail: &'static str,
) {
    let Some(names) = env::var_os(variable) else {
        return;
    };
    for name in names.to_string_lossy().split_whitespace() {
        if !name.is_empty() && !(detail == "function" && name.starts_with('_')) {
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
