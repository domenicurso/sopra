use std::path::PathBuf;

use super::complete;
use crate::{
    Request,
    filesystem::FilesystemEngine,
    graph::{CommandNode, PositionalSpec, ValueSpec},
    parser::Invocation,
};

#[test]
fn help_choices_reach_pending_option_completion() {
    let graph = crate::parser::help::parse(
        "Options:\n  --edition <YEAR>  Edition [possible values: 2015, 2018, 2021, 2024]\n",
        "cargo",
    );
    let invocation = Invocation {
        key: "cargo".to_string(),
        command: None,
        program: "cargo".to_string(),
        args: vec!["--edition".to_string(), "202".to_string()],
        active: "202".to_string(),
        trailing_space: false,
    };
    let request = Request {
        line: "cargo --edition 202".to_string(),
        cursor: 19,
        context_line: String::new(),
        context_cursor: 0,
        replace: 16..19,
        cwd: PathBuf::from("."),
        generation: 0,
        context_key: String::new(),
    };
    let mut filesystem = FilesystemEngine::new();
    let items = complete(&graph.root, &invocation, &request, &mut filesystem);
    let names = items
        .iter()
        .map(|item| item.insert.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, ["2021", "2024"]);
}

#[test]
fn active_positional_is_filtered_as_the_current_value() {
    let mut value = ValueSpec::named("status");
    value.choices.push("status".to_string());
    let node = CommandNode {
        positionals: vec![PositionalSpec {
            name: "status".to_string(),
            description: None,
            value,
            optional: false,
            repeatable: false,
        }],
        ..CommandNode::named("tool")
    };
    let invocation = Invocation {
        key: "tool".to_string(),
        command: None,
        program: "tool".to_string(),
        args: vec!["s".to_string()],
        active: "s".to_string(),
        trailing_space: false,
    };
    let request = Request {
        line: "tool s".to_string(),
        cursor: 6,
        context_line: String::new(),
        context_cursor: 0,
        replace: 5..6,
        cwd: PathBuf::from("."),
        generation: 0,
        context_key: String::new(),
    };
    let mut filesystem = FilesystemEngine::new();
    let items = complete(&node, &invocation, &request, &mut filesystem);
    assert_eq!(
        items
            .iter()
            .map(|item| item.display.as_str())
            .collect::<Vec<_>>(),
        ["status"]
    );
}

#[test]
fn choice_queries_support_fuzzy_abbreviations() {
    assert!(super::values::matches_query("read-write", "rw"));
}

#[test]
fn assignment_values_complete_keys_before_values() {
    let mut value = ValueSpec::named("setting");
    value.choices = ["status=private", "status=public", "mfa=none"]
        .into_iter()
        .map(str::to_string)
        .collect();
    let node = CommandNode {
        positionals: vec![PositionalSpec {
            name: "setting".to_string(),
            description: None,
            value,
            optional: false,
            repeatable: false,
        }],
        ..CommandNode::named("tool")
    };
    let invocation = Invocation {
        key: "tool".to_string(),
        command: None,
        program: "tool".to_string(),
        args: Vec::new(),
        active: String::new(),
        trailing_space: true,
    };
    let request = Request {
        line: "tool ".to_string(),
        cursor: 5,
        context_line: String::new(),
        context_cursor: 0,
        replace: 5..5,
        cwd: PathBuf::from("."),
        generation: 0,
        context_key: String::new(),
    };
    let mut filesystem = FilesystemEngine::new();
    let items = complete(&node, &invocation, &request, &mut filesystem);
    assert_eq!(
        items
            .iter()
            .map(|item| item.display.as_str())
            .collect::<Vec<_>>(),
        ["mfa=", "status="]
    );
}

#[test]
fn assignment_values_replace_only_the_value_suffix() {
    let mut value = ValueSpec::named("setting");
    value.choices = ["status=private", "status=public"]
        .into_iter()
        .map(str::to_string)
        .collect();
    let node = CommandNode {
        positionals: vec![PositionalSpec {
            name: "setting".to_string(),
            description: None,
            value,
            optional: false,
            repeatable: false,
        }],
        ..CommandNode::named("tool")
    };
    let invocation = Invocation {
        key: "tool".to_string(),
        command: None,
        program: "tool".to_string(),
        args: vec!["status=".to_string()],
        active: "status=".to_string(),
        trailing_space: false,
    };
    let request = Request {
        line: "tool status=".to_string(),
        cursor: 12,
        context_line: String::new(),
        context_cursor: 0,
        replace: 5..12,
        cwd: PathBuf::from("."),
        generation: 0,
        context_key: String::new(),
    };
    let mut filesystem = FilesystemEngine::new();
    let items = complete(&node, &invocation, &request, &mut filesystem);
    assert_eq!(
        items
            .iter()
            .map(|item| (item.display.as_str(), item.insert.as_str(), &item.replace))
            .collect::<Vec<_>>(),
        [
            ("private", "private", &(12..12)),
            ("public", "public", &(12..12))
        ]
    );
}
