use super::super::{matches_path, parse, parse_for};

#[test]
fn wrapped_words_are_descriptions_and_parameterized_rows_are_commands() {
    let graph = parse(
        "Usage: tool <command>\n\nCommands\n  doctor              Diagnose the worker and Rust\n                      compiler\n  dev <command>       Develop a checkout\n  debug <command>     Inspect graphs\n  test [args ...]     Run tests\n  widgets list        List widgets\n\nNotes:\nExamples follow here.\n",
        "tool",
    );
    let names = graph
        .root
        .subcommands
        .iter()
        .map(|node| node.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, ["doctor", "dev", "debug", "test", "widgets"]);
    assert_eq!(
        graph.root.subcommands[0].description.as_deref(),
        Some("Diagnose the worker and Rust compiler")
    );
    assert_eq!(
        graph.root.subcommands[2].description.as_deref(),
        Some("Inspect graphs")
    );
    assert_eq!(graph.root.subcommands[3].positionals[0].name, "args");
    assert!(graph.root.subcommands[3].positionals[0].repeatable);
    assert_eq!(graph.root.subcommands[4].subcommands[0].name, "list");
}

#[test]
fn diagnostic_only_output_is_not_a_help_source() {
    for text in [
        "tool: unknown command or arguments; run `tool --help`",
        "Unable to load credentials",
        "error: invalid option --help",
    ] {
        assert!(!parse(text, "tool").useful(), "{text}");
    }
}

#[test]
fn renamed_executables_use_the_documented_invocation_and_reject_parent_help() {
    let text = "Usage: tool dev <command>\n\nCommands:\n  build  Build the project\n  check  Check the project\n";
    let path = vec!["dev".to_string()];
    assert!(matches_path(text, "tool-worker", &path));
    assert!(!matches_path(
        text,
        "tool-worker",
        &["dev".into(), "build".into()]
    ));
    let graph = parse_for(text, "tool-worker", &path);
    assert_eq!(graph.root.name, "tool-worker");
    assert_eq!(graph.root.subcommands.len(), 2);
    assert!(graph.root.positionals.is_empty());
}

#[test]
fn unadorned_usage_after_a_heading_is_still_a_synopsis() {
    let graph = parse("Useful summary\n\nUsage:\ntool run\n", "tool");
    assert!(graph.root.synopsis.is_some());
    assert_eq!(graph.root.description.as_deref(), Some("Useful summary"));
}
