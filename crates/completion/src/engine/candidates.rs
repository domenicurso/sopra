use std::{collections::HashSet, ops::Range};

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
        || candidate
            .to_ascii_lowercase()
            .contains(&query.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::complete;
    use crate::{Request, filesystem::FilesystemEngine, parser::Invocation};

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
}
