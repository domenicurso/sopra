use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    io,
    path::{Path, PathBuf},
};

const STORES: &[(&str, &str)] = &[
    ("Sources", "completion-cache"),
    ("Graphs", "completion-graphs"),
];

pub fn status() -> io::Result<String> {
    status_at(&std::env::temp_dir().join("sopra"))
}

pub fn clear(command: Option<&str>) -> io::Result<String> {
    clear_at(&std::env::temp_dir().join("sopra"), command)
}

pub fn validate_command(command: &str) -> io::Result<()> {
    if command.is_empty()
        || command.chars().any(char::is_control)
        || Path::new(command).file_name().is_none()
    {
        return Err(io::Error::other("supply a command name or executable path"));
    }
    Ok(())
}

pub(crate) fn command_directory(store: &str, command: &str) -> PathBuf {
    directory(&std::env::temp_dir().join("sopra"), store, command)
}

fn directory(base: &Path, store: &str, command: &str) -> PathBuf {
    let mut hash = DefaultHasher::new();
    Path::new(command).file_name().hash(&mut hash);
    base.join(store).join(format!("{:016x}", hash.finish()))
}

fn status_at(base: &Path) -> io::Result<String> {
    let mut output = String::new();
    for (label, store) in STORES {
        let path = base.join(store);
        let files = inventory(&path)?;
        let bytes: u64 = files.iter().map(|(_, size)| size).sum();
        output.push_str(&format!(
            "{label}: {} files, {bytes} bytes\n  {}\n",
            files.len(),
            path.display()
        ));
    }
    Ok(output)
}

fn clear_at(base: &Path, command: Option<&str>) -> io::Result<String> {
    if let Some(command) = command {
        validate_command(command)?;
    }
    let mut removed = 0;
    let mut bytes = 0;
    for (_, store) in STORES {
        let path = command.map_or_else(|| base.join(store), |name| directory(base, store, name));
        let files = inventory(&path)?;
        for (file, size) in files {
            match fs::remove_file(file) {
                Ok(()) => {
                    removed += 1;
                    bytes += size;
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        prune_empty(&path)?;
        if command.is_some() {
            remove_empty(&base.join(store))?;
        }
    }
    remove_empty(base)?;
    let scope = command.map_or_else(|| "all commands".into(), |name| format!("{name:?}"));
    Ok(format!(
        "Cleared completion cache for {scope}: {removed} files, {bytes} bytes.\n"
    ))
}

fn prune_empty(path: &Path) -> io::Result<()> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            prune_empty(&entry.path())?;
        }
    }
    remove_empty(path)
}

fn remove_empty(path: &Path) -> io::Result<()> {
    match fs::remove_dir(path) {
        Ok(()) => Ok(()),
        // Another shell can write new cache data while this shell is clearing it.
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::DirectoryNotEmpty
            ) =>
        {
            Ok(())
        }
        Err(error) => Err(error),
    }
}

pub(crate) fn inventory(path: &Path) -> io::Result<Vec<(PathBuf, u64)>> {
    let mut files = Vec::new();
    collect(path, 0, &mut files)?;
    Ok(files)
}

fn collect(path: &Path, depth: usize, files: &mut Vec<(PathBuf, u64)>) -> io::Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if metadata.is_symlink() {
        return Err(io::Error::other(format!(
            "cache path is a symlink: {}",
            path.display()
        )));
    }
    if metadata.is_file() {
        files.push((path.into(), metadata.len()));
    } else if metadata.is_dir() && depth < 2 {
        for entry in fs::read_dir(path)? {
            collect(&entry?.path(), depth + 1, files)?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
