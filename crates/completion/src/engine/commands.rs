use crate::{
    CompletionItem, CompletionKind, CompletionSource,
    graph::{CommandNode, OptionSpec},
};
use std::{collections::HashSet, ops::Range};

pub(super) fn options(
    node: &CommandNode,
    invocation: &crate::parser::Invocation,
    replace: Range<usize>,
) -> Vec<CompletionItem> {
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    let selected = super::arguments::selected(node, invocation);
    for option in &node.options {
        if !available(node, option, &selected) {
            continue;
        }
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

pub(super) fn items(commands: &[CommandNode], replace: Range<usize>) -> Vec<CompletionItem> {
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
    let (invocation, leaf) = invocation(name, command);
    let mut item = CompletionItem::with_range(
        &invocation,
        &invocation,
        leaf.description
            .clone()
            .or_else(|| command.description.clone()),
        CompletionKind::Subcommand,
        replace,
        CompletionSource::CommandIndex,
    );
    item.group = Some("Commands".to_string());
    item
}

fn invocation<'a>(name: &str, mut node: &'a CommandNode) -> (String, &'a CommandNode) {
    let mut words = vec![name.to_string()];
    // A grouping node with one action needs no additional menu decision.
    while node.subcommands.len() == 1 && node.options.is_empty() && node.positionals.is_empty() {
        node = &node.subcommands[0];
        words.push(node.name.clone());
    }
    (words.join(" "), node)
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;

fn available(node: &CommandNode, option: &OptionSpec, selected: &[&OptionSpec]) -> bool {
    if !option.repeatable && selected.iter().any(|used| used.names == option.names) {
        return false;
    }
    !node.exclusive.iter().any(|group| {
        option.names.iter().any(|name| group.contains(name))
            && selected.iter().any(|used| {
                used.names != option.names && used.names.iter().any(|name| group.contains(name))
            })
    })
}
