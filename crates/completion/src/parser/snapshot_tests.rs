use super::*;
use crate::{Request, engine::LocalCompletion, parser};
use std::{fs, time::Instant};

#[test]
fn restart_and_directory_changes_serve_snapshots_without_waiting_for_a_probe()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::temp_dir().join(format!("sopra-frame-{}", std::process::id()));
    fs::create_dir_all(&directory)?;
    let executable = directory.join("frame-cli");
    fs::write(&executable, "#!/bin/sh\nsleep 5\n")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))?;
    }
    let line = format!("{} ", executable.display());
    let mut request = Request {
        line: line.clone(),
        cursor: line.chars().count(),
        context_line: line.clone(),
        context_cursor: line.chars().count(),
        replace: line.len()..line.len(),
        cwd: directory.clone(),
        generation: 1,
        context_key: String::new(),
    };
    let invocation = parser::invocation(&request).ok_or("fixture resolves")?;
    let help = parser::request_for(&request, &invocation);
    let graph = parser::help::parse("Commands:\n  ready  Available this frame\n", "frame-cli");
    write(&help, &graph);
    for changed_directory in [false, true] {
        if changed_directory {
            request.cwd = directory.join("nested");
            fs::create_dir_all(&request.cwd)?;
        }
        let mut local = LocalCompletion::new();
        let started = Instant::now();
        let result = local.complete(&request);
        let elapsed = started.elapsed();
        assert!(result.items.iter().any(|item| item.display == "ready"));
        assert!(
            elapsed < Duration::from_millis(16),
            "snapshot missed a 60Hz frame: {elapsed:?}"
        );
        assert_eq!(result.help.is_some(), changed_directory);
        eprintln!("snapshot immediate frame ({changed_directory} directory change): {elapsed:?}");
    }
    verify_corruption(&help)?;
    fs::remove_dir_all(directory)?;
    Ok(())
}

fn verify_corruption(help: &HelpRequest) -> std::io::Result<()> {
    let file = path(help, false);
    fs::write(&file, b"corrupt")?;
    assert!(
        read(help).is_some(),
        "valid provisional snapshot survives an exact-file corruption"
    );
    fs::write(path(help, true), b"corrupt")?;
    assert!(read(help).is_none());
    Ok(())
}
