use std::path::PathBuf;

use super::{CompletionEngine, CompletionResponseSource, help};

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
    assert!(engine.pending_context.is_none());
    let response = engine.poll().next().expect("local completion");
    assert_eq!(response.source, CompletionResponseSource::Local);
}

#[test]
fn help_parser_is_command_agnostic() {
    let spec = help::parse_help(
        "Commands:\n  tool open <file>  open a file\n  tool sync         synchronize data\n\nOptions:\n  -q, --quiet       suppress output [boolean]\n      --format      output format [string] [choices: \"json\", \"text\"]\n",
    );
    assert!(spec.commands.iter().any(|command| command.name == "open"));
    assert!(spec.commands.iter().any(|command| command.name == "sync"));
    assert!(
        spec.options
            .iter()
            .any(|option| option.names == ["-q", "--quiet"])
    );
    let format = spec
        .options
        .iter()
        .find(|option| option.names.iter().any(|name| name == "--format"))
        .expect("format option");
    assert_eq!(format.values, ["json", "text"]);
}

#[test]
fn whitespace_does_not_request_the_root_command_menu() {
    let mut engine = CompletionEngine::new().expect("completion worker");
    engine.request("   ", 3, PathBuf::from(".").as_path());
    assert!(engine.poll().next().is_none());
}
