use super::*;

fn fixture() -> io::Result<PathBuf> {
    static SERIAL: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let serial = SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let base = std::env::temp_dir().join(format!(
        "sopra-cache-controls-{}-{serial}",
        std::process::id()
    ));
    fs::create_dir_all(&base)?;
    Ok(base)
}

#[test]
fn targeted_clear_preserves_other_commands_and_full_clear_removes_legacy_files() -> io::Result<()> {
    let base = fixture()?;
    for (_, store) in STORES {
        for command in ["git", "npm"] {
            let path = directory(&base, store, command);
            fs::create_dir_all(&path)?;
            fs::write(path.join("first"), b"abc")?;
            fs::write(path.join("nested-context"), b"def")?;
        }
        fs::write(base.join(store).join("legacy"), b"old")?;
    }
    let report = status_at(&base)?;
    assert!(report.contains("Sources: 5 files, 15 bytes"), "{report}");
    assert!(report.contains("Graphs: 5 files, 15 bytes"), "{report}");
    assert!(clear_at(&base, Some("/usr/local/bin/git"))?.contains("4 files, 12 bytes"));
    assert_eq!(
        inventory(&directory(&base, "completion-cache", "npm"))?.len(),
        2
    );
    assert!(clear_at(&base, Some("git"))?.contains("0 files, 0 bytes"));
    assert!(clear_at(&base, None)?.contains("6 files, 18 bytes"));
    assert!(status_at(&base)?.contains("Sources: 0 files, 0 bytes"));
    fs::write(base.join("unrelated"), b"keep")?;
    clear_at(&base, None)?;
    assert!(base.join("unrelated").exists());
    fs::remove_dir_all(base)
}

#[test]
fn missing_stores_are_empty_and_invalid_scopes_are_rejected() -> io::Result<()> {
    let base = fixture()?;
    assert!(status_at(&base)?.contains("0 files, 0 bytes"));
    for name in ["", "/", "..", "bad\ncommand", "bad\tcommand"] {
        assert!(clear_at(&base, Some(name)).is_err(), "{name:?}");
    }
    fs::remove_dir_all(base)
}

#[cfg(unix)]
#[test]
fn clear_refuses_symlinks_instead_of_deleting_external_data() -> io::Result<()> {
    let base = fixture()?;
    let outside = base.join("outside");
    fs::create_dir_all(&outside)?;
    fs::write(outside.join("keep"), b"value")?;
    fs::create_dir_all(base.join("completion-cache"))?;
    std::os::unix::fs::symlink(&outside, directory(&base, "completion-cache", "git"))?;
    assert!(clear_at(&base, Some("git")).is_err());
    assert_eq!(fs::read(outside.join("keep"))?, b"value");
    fs::remove_dir_all(base)
}
