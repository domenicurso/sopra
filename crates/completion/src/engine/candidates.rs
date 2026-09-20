use std::{collections::HashSet, ops::Range};

use neo_frizbee::{Config, Matcher};

use crate::{
    CompletionItem, CompletionKind, CompletionSource, Request,
    filesystem::FilesystemEngine,
    graph::{CommandNode, OptionSpec, ValueKind, ValueSpec},
    parser::Invocation,
};

use super::arguments::{next_positional, pending_value};

pub(super) fn complete(
    node: &CommandNode,
    invocation: &Invocation,
    request: &Request,
    filesystem: &mut FilesystemEngine,
) -> Vec<CompletionItem> {
    if let Some((value, query, replace)) = pending_value(node, invocation, request) {
        return value_items(value, &query, replace, filesystem, request);
    }
    if invocation.active.starts_with('-') {
        return option_items(&node.options, request.replace.clone());
    }
    let positional = next_positional(node, invocation);
    if positional.is_some_and(|value| prefers_value(value, &invocation.active)) {
        return value_items(
            &positional.expect("checked positional").value,
            &invocation.active,
            request.replace.clone(),
            filesystem,
            request,
        );
    }
    if is_partial_subcommand(invocation) {
        return command_items(&node.subcommands, request.replace.clone());
    }
    if invocation.trailing_space && !node.subcommands.is_empty() {
        return command_items(&node.subcommands, request.replace.clone());
    }
    let Some(positional) = positional else {
        return command_items(&node.subcommands, request.replace.clone());
    };
    value_items(
        &positional.value,
        &invocation.active,
        request.replace.clone(),
        filesystem,
        request,
    )
}

fn prefers_value(positional: &crate::graph::PositionalSpec, query: &str) -> bool {
    !positional.value.choices.is_empty()
        || matches!(
            positional.value.kind,
            ValueKind::File | ValueKind::Directory
        )
        || query.starts_with(['/', '.', '~'])
        || query.contains('/')
}

fn value_items(
    value: &ValueSpec,
    query: &str,
    replace: Range<usize>,
    filesystem: &mut FilesystemEngine,
    request: &Request,
) -> Vec<CompletionItem> {
    if !value.choices.is_empty() {
        if let Some(items) = assignment_items(&value.choices, query, replace.clone()) {
            return items;
        }
        return value
            .choices
            .iter()
            .filter(|choice| matches_query(choice, query))
            .map(|choice| {
                CompletionItem::with_range(
                    choice,
                    choice,
                    None,
                    CompletionKind::Value,
                    replace.clone(),
                    CompletionSource::CommandIndex,
                )
            })
            .collect();
    }
    match value.kind {
        ValueKind::File | ValueKind::Directory => filesystem.complete(request, query),
        _ => Vec::new(),
    }
}

fn assignment_items(
    choices: &[String],
    query: &str,
    replace: Range<usize>,
) -> Option<Vec<CompletionItem>> {
    let assignments = choices
        .iter()
        .map(|choice| choice.split_once('='))
        .collect::<Option<Vec<_>>>()?;
    if assignments.iter().any(|(key, _)| key.is_empty()) {
        return None;
    }

    if let Some((key, value_query)) = query.split_once('=') {
        let prefix = format!("{key}=");
        let value_replace = replace.start + prefix.len()..replace.end;
        return Some(
            assignments
                .iter()
                .filter(|(candidate_key, value)| {
                    *candidate_key == key && matches_query(value, value_query)
                })
                .map(|(_, value)| {
                    CompletionItem::with_range(
                        *value,
                        *value,
                        None,
                        CompletionKind::Value,
                        value_replace.clone(),
                        CompletionSource::CommandIndex,
                    )
                })
                .collect(),
        );
    }

    let mut keys = assignments
        .iter()
        .map(|(key, _)| format!("{key}="))
        .collect::<Vec<_>>();
    keys.sort_unstable();
    keys.dedup();
    Some(
        keys.into_iter()
            .filter(|key| matches_query(key, query))
            .map(|key| {
                CompletionItem::with_range(
                    &key,
                    &key,
                    None,
                    CompletionKind::Value,
                    replace.clone(),
                    CompletionSource::CommandIndex,
                )
            })
            .collect(),
    )
}

fn option_items(options: &[OptionSpec], replace: Range<usize>) -> Vec<CompletionItem> {
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    for option in options {
        for name in &option.names {
            if seen.insert(name.as_str()) {
                items.push(option_item(name, option, replace.clone()));
            }
        }
    }
    items
}

fn option_item(name: &str, option: &OptionSpec, replace: Range<usize>) -> CompletionItem {
    let mut item = CompletionItem::with_range(
        name,
        name,
        option.description.clone(),
        CompletionKind::Option,
        replace,
        CompletionSource::CommandIndex,
    );
    item.group = Some("Options".to_string());
    item
}

fn command_items(commands: &[CommandNode], replace: Range<usize>) -> Vec<CompletionItem> {
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    for command in commands {
        for name in std::iter::once(&command.name).chain(command.aliases.iter()) {
            if seen.insert(name.as_str()) {
                items.push(command_item(name, command, replace.clone()));
            }
        }
    }
    items
}

fn command_item(name: &str, command: &CommandNode, replace: Range<usize>) -> CompletionItem {
    let mut item = CompletionItem::with_range(
        name,
        name,
        command.description.clone(),
        CompletionKind::Subcommand,
        replace,
        CompletionSource::CommandIndex,
    );
    item.group = Some("Commands".to_string());
    item
}

fn is_partial_subcommand(invocation: &Invocation) -> bool {
    invocation.args.len() == 1
        && !invocation.trailing_space
        && invocation.args[0] == invocation.active
}

fn matches_query(candidate: &str, query: &str) -> bool {
    query.is_empty()
        || Matcher::new(query, &Config::default())
            .match_one_indices(candidate, 0)
            .is_some()
}

#[cfg(test)]
mod tests {
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
        assert!(super::matches_query("read-write", "rw"));
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
}
