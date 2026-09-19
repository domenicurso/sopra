use std::path::PathBuf;

use super::{CompletionEngine, CompletionResponseSource, parser};

#[test]
fn local_completion_refreshes_inside_a_cached_semantic_context() {
    let mut engine = CompletionEngine::new().expect("completion worker");
    let cwd = PathBuf::from(".");
    engine.request("cd ", 3, &cwd);
    engine.request("cd C", 4, &cwd);
    let responses = engine.poll().collect::<Vec<_>>();
    assert_eq!(
        responses
            .iter()
            .filter(|response| response.source == CompletionResponseSource::Local)
            .count(),
        2
    );
}

#[test]
fn command_name_completion_does_not_queue_a_second_engine() {
    let mut engine = CompletionEngine::new().expect("completion worker");
    engine.request("e", 1, PathBuf::from(".").as_path());
    let response = engine.poll().next().expect("local completion");
    assert_eq!(response.source, CompletionResponseSource::Local);
}

#[test]
fn help_parser_is_command_agnostic() {
    let graph = parser::help::parse(
        "Commands:\n  tool open <file>  open a file\n  tool sync         synchronize data\n\nOptions:\n  -q, --quiet       suppress output [boolean]\n      --format      output format [string] [choices: \"json\", \"text\"]\n",
        "tool",
    );
    assert!(
        graph
            .root
            .subcommands
            .iter()
            .any(|command| command.name == "open")
    );
    assert!(
        graph
            .root
            .subcommands
            .iter()
            .any(|command| command.name == "sync")
    );
    assert!(
        graph
            .root
            .options
            .iter()
            .any(|option| option.names == ["-q", "--quiet"])
    );
    let format = graph
        .root
        .options
        .iter()
        .find(|option| option.names.iter().any(|name| name == "--format"))
        .expect("format option");
    assert_eq!(
        format.value.as_ref().expect("format value").choices,
        ["json", "text"]
    );
}

#[test]
fn whitespace_does_not_request_the_root_command_menu() {
    let mut engine = CompletionEngine::new().expect("completion worker");
    engine.request("   ", 3, PathBuf::from(".").as_path());
    assert!(engine.poll().next().is_none());
}

#[cfg(unix)]
#[test]
fn help_response_hydrates_nested_command_before_replaying_completion() {
    use std::{fs, os::unix::fs::PermissionsExt, thread, time::Duration};

    let path = std::env::temp_dir().join(format!(
        "sopra-completion-fixture-{}",
        std::process::id()
    ));
    fs::write(
        &path,
        "#!/bin/sh\nif [ \"$1\" = child ]; then\n  printf 'child\\n\\nOptions:\\n  --child-option  child option\\n'\nelse\n  printf 'Commands:\\n  child  child command\\n\\nOptions:\\n  --root-option  root option\\n'\nfi\n",
    )
    .expect("fixture script");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("fixture executable");

    let line = format!("{} child --", path.display());
    let mut engine = CompletionEngine::new().expect("completion worker");
    engine.request(&line, line.chars().count(), PathBuf::from(".").as_path());
    let mut found = false;
    for _ in 0..100 {
        found |= engine
            .poll()
            .flat_map(|response| response.items)
            .any(|item| item.display == "--child-option");
        if found {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    let _ = fs::remove_file(path);
    assert!(found);
}
