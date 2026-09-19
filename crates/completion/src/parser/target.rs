use crate::graph::{CommandGraph, CommandNode, ValueAttachment};

use super::context::Invocation;

pub(crate) fn resolve<'a>(
    graph: &'a CommandGraph,
    invocation: &Invocation,
) -> (&'a CommandNode, Invocation, Vec<String>) {
    let (node, start, path) = target_node(graph, invocation);
    let mut nested = invocation.clone();
    nested.args = invocation.args[start..].to_vec();
    (node, nested, path)
}

fn target_node<'a>(
    graph: &'a CommandGraph,
    invocation: &Invocation,
) -> (&'a CommandNode, usize, Vec<String>) {
    let mut node = &graph.root;
    let mut start = 0;
    let mut index = 0;
    let mut path = Vec::new();
    while index < invocation.args.len() {
        let argument = &invocation.args[index];
        if is_active(invocation, index, argument) {
            break;
        }
        if argument == "--" {
            break;
        }
        if argument.starts_with('-') {
            index += if option_takes_value(node, argument) {
                2
            } else {
                1
            };
            continue;
        }
        let Some(child) = node.subcommands.iter().find(|child| {
            child.name == *argument || child.aliases.iter().any(|alias| alias == argument)
        }) else {
            break;
        };
        path.push(child.name.clone());
        node = child;
        start = index + 1;
        index += 1;
    }
    (node, start, path)
}

fn is_active(invocation: &Invocation, index: usize, argument: &str) -> bool {
    !invocation.trailing_space
        && index + 1 == invocation.args.len()
        && argument == invocation.active
}

fn option_takes_value(node: &CommandNode, argument: &str) -> bool {
    if argument.contains('=') {
        return false;
    }
    let Some(option) = node
        .options
        .iter()
        .find(|option| option.names.iter().any(|name| name == argument))
    else {
        return false;
    };
    option.value.is_some() && option.attachment != ValueAttachment::Attached
}
