use super::parse;
use crate::graph::{ValueAttachment, ValueKind};

mod corpus;

#[test]
fn parses_nested_grep_synopsis_details() {
    let text = r#"usage: grep [-abcdDEFGHhIiJLlMmnOopqRSsUVvwXxZz] [-A num] [-B num] [-C[num]]
    [-e pattern] [-f file] [--binary-files=value] [--color=when]
    [--context[=num]] [--directories=action] [--label] [pattern] [file ...]

Options:
  -A num                  print num lines of trailing context
  -C[num], --context[=num]  print num lines of context
  --color=when            colorize the match
"#;
    let graph = parse(text, "grep");
    let amount = graph
        .root
        .options
        .iter()
        .find(|option| option.names == ["-A"])
        .expect("-A option");
    assert_eq!(
        amount.value.as_ref().map(|value| value.name.as_str()),
        Some("num")
    );
    assert!(!amount.value.as_ref().expect("-A value").optional);
    let context = graph
        .root
        .options
        .iter()
        .find(|option| option.names.iter().any(|name| name == "--context"))
        .expect("context option");
    assert!(context.value.as_ref().expect("context value").optional);
    assert_eq!(context.attachment, ValueAttachment::Either);
    assert!(graph.root.options.iter().any(|option| {
        option.names == ["-a"] && option.names.iter().all(|name| name.len() == 2)
    }));
    assert!(graph.root.positionals.iter().any(|positional| {
        positional.name == "file"
            && positional.repeatable
            && positional.value.kind == ValueKind::File
    }));
    assert!(graph.root.synopsis.is_some());
}

#[test]
fn merges_commands_and_aliases_from_framework_help() {
    let graph = parse(
        "Commands:\n  tool open <file>  open a file [aliases: o]\n  tool sync         synchronize data\n\nOptions:\n  -q, --quiet       suppress output\n",
        "tool",
    );
    let open = graph
        .root
        .subcommands
        .iter()
        .find(|command| command.name == "open")
        .expect("open command");
    assert_eq!(open.aliases, ["o"]);
    assert!(
        graph
            .root
            .options
            .iter()
            .any(|option| option.names == ["-q", "--quiet"])
    );
}

#[test]
fn parses_argparse_choice_subcommands_and_options() {
    let graph = parse(
        "usage: tool [-h] {run,init}\n\npositional arguments:\n  {run,init}\n    run       execute a task\n    init      create a project\n\noptions:\n  -h, --help  show this help message and exit\n",
        "tool",
    );
    assert!(graph.root.subcommands.iter().any(|command| {
        command.name == "run" && command.description.as_deref() == Some("execute a task")
    }));
    assert!(
        graph
            .root
            .options
            .iter()
            .any(|option| option.names == ["-h", "--help"])
    );
}
