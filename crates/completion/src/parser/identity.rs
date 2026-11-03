use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::Path,
};

// A long-lived worker must stop reusing graphs after a rebuild or directory change.
pub(crate) fn key(resolved: &str, executable: Option<&Path>, cwd: &Path) -> String {
    let mut hash = DefaultHasher::new();
    cwd.hash(&mut hash);
    for name in [
        "PATH",
        "MANPATH",
        "LANG",
        "LC_ALL",
        "HOME",
        "XDG_CONFIG_HOME",
    ] {
        sopra_shell_context::var_os(name).hash(&mut hash);
    }
    if let Some(metadata) = executable.and_then(|path| fs::metadata(path).ok()) {
        metadata.len().hash(&mut hash);
        metadata.modified().ok().hash(&mut hash);
    }
    format!("{resolved}\0{:016x}", hash.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graphs_follow_rebuilds_and_working_directories() -> std::io::Result<()> {
        let path = std::env::temp_dir().join(format!("sopra-identity-{}", std::process::id()));
        fs::write(&path, "old executable")?;
        let before = key("tool", Some(&path), Path::new("/first"));
        fs::write(&path, "replacement executable")?;
        assert_ne!(before, key("tool", Some(&path), Path::new("/first")));
        assert_ne!(before, key("tool", Some(&path), Path::new("/second")));
        fs::remove_file(path)
    }
}
