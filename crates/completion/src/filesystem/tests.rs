use std::{fs, path::PathBuf, time::SystemTime};

use super::{FilesystemEngine, ParsedPath};
use crate::Request;

#[test]
fn parsed_paths_keep_shell_prefixes() {
    let path = ParsedPath::parse("--file=./src/ed", PathBuf::from("/tmp").as_path()).expect("path");
    assert_eq!(path.option_prefix, "--file=");
    assert_eq!(path.display_prefix, "./");
    assert_eq!(path.segments, ["src"]);
    assert_eq!(path.leaf, "ed");
}

#[test]
fn relative_paths_do_not_gain_a_leading_separator() {
    let path = ParsedPath::parse("src/ma", PathBuf::from("/tmp").as_path()).expect("path");
    assert_eq!(path.render(&path.segments, "main.rs", false), "src/main.rs");
}

#[test]
fn quoted_and_escaped_paths_keep_their_shell_shape() {
    let quoted =
        ParsedPath::parse("--file='src/ma'", PathBuf::from("/tmp").as_path()).expect("quoted path");
    assert_eq!(quoted.leaf, "ma");
    assert_eq!(
        quoted.render(&quoted.segments, "main file", false),
        "--file='src/main file'"
    );

    let escaped =
        ParsedPath::parse("src/ma\\ x", PathBuf::from("/tmp").as_path()).expect("escaped path");
    assert_eq!(escaped.leaf, "ma x");
}

#[test]
fn beam_search_keeps_multiple_fuzzy_directory_branches() {
    let root = std::env::temp_dir().join(format!(
        "fs-beam-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(root.join("u1/delta/docs")).expect("first branch");
    fs::create_dir_all(root.join("u2/data/docs")).expect("second branch");
    fs::write(root.join("u1/delta/docs/object.txt"), b"one").expect("first leaf");
    fs::write(root.join("u2/data/docs/object.txt"), b"two").expect("second leaf");

    let token = "u/d/d/ob";
    let mut engine = FilesystemEngine::new();
    let response = engine.complete(
        &Request {
            line: token.to_string(),
            cursor: token.len(),
            context_line: token.to_string(),
            context_cursor: token.len(),
            replace: 0..token.len(),
            cwd: root.clone(),
            generation: 0,
            context_key: String::new(),
        },
        token,
    );
    let inserts = response
        .iter()
        .map(|item| item.insert.as_str())
        .collect::<Vec<_>>();
    assert!(inserts.contains(&"u1/delta/docs/object.txt"));
    assert!(inserts.contains(&"u2/data/docs/object.txt"));
    drop(engine);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn fuzzy_root_paths_keep_the_full_shell_path() {
    let root = std::env::temp_dir().join(format!(
        "fs-root-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(root.join("Applications/Accents.app")).expect("applications");

    let token = "a/";
    let mut engine = FilesystemEngine::new();
    let response = engine.complete(
        &Request {
            line: token.to_string(),
            cursor: token.len(),
            context_line: token.to_string(),
            context_cursor: token.len(),
            replace: 0..token.len(),
            cwd: root.clone(),
            generation: 0,
            context_key: String::new(),
        },
        token,
    );
    assert!(
        response.iter().any(|item| {
            item.insert == "Applications/Accents.app/" && item.display == item.insert
        }),
        "{response:?}"
    );
    drop(engine);
    fs::remove_dir_all(root).expect("cleanup");
}
