use std::path::PathBuf;

use super::LocalCompletion;
use crate::Request;

#[test]
fn filesystem_completion_uses_the_explicit_token_range() {
    let mut local = LocalCompletion::new();
    let request = Request {
        line: "cd ./".to_string(),
        cursor: 5,
        context_line: "cd ./".to_string(),
        context_cursor: 5,
        replace: 3..5,
        cwd: PathBuf::from("."),
        generation: 0,
        context_key: String::new(),
    };
    for item in local.complete(&request).items {
        assert_eq!(item.replace, 3..5);
    }
}

#[test]
fn unresolved_commands_offer_files_without_a_command_name_allowlist() -> std::io::Result<()> {
    let cwd = std::env::temp_dir().join(format!("sopra-fallback-{}", std::process::id()));
    std::fs::create_dir_all(&cwd)?;
    std::fs::write(cwd.join("notes.txt"), "fixture")?;
    let line = "sopra-unknown-command notes";
    let request = Request {
        line: line.into(),
        cursor: line.len(),
        context_line: line.into(),
        context_cursor: line.len(),
        replace: 22..line.len(),
        cwd: cwd.clone(),
        generation: 1,
        context_key: String::new(),
    };
    let items = LocalCompletion::new().complete(&request).items;
    std::fs::remove_dir_all(cwd)?;
    assert!(items.iter().any(|item| item.display == "notes.txt"));
    Ok(())
}
