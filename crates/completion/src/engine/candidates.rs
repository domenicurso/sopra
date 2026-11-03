use super::{
    arguments::{next_positional, pending_value},
    commands, values,
};
use crate::{
    CompletionItem, Request,
    filesystem::FilesystemEngine,
    graph::{CommandNode, ValueKind},
    parser::Invocation,
};

pub(super) fn complete(
    node: &CommandNode,
    invocation: &Invocation,
    request: &Request,
    filesystem: &mut FilesystemEngine,
) -> Vec<CompletionItem> {
    if let Some((value, query, replace)) = pending_value(node, invocation, request) {
        return values::items(value, (&query, replace), filesystem, request);
    }
    let terminated = invocation
        .args
        .iter()
        .take(super::arguments::completed_count(invocation))
        .any(|arg| arg == "--");
    if invocation.active.starts_with('-') && !terminated {
        return commands::options(node, invocation, request.replace.clone());
    }
    let positional = next_positional(node, invocation);
    if positional.is_some_and(|value| prefers_value(value, &invocation.active)) {
        return values::items(
            &positional.expect("checked positional").value,
            (&invocation.active, request.replace.clone()),
            filesystem,
            request,
        );
    }
    if is_partial_subcommand(invocation) {
        return commands::items(&node.subcommands, request.replace.clone());
    }
    if invocation.trailing_space && !node.subcommands.is_empty() {
        return commands::items(&node.subcommands, request.replace.clone());
    }
    let Some(positional) = positional else {
        return commands::items(&node.subcommands, request.replace.clone());
    };
    values::items(
        &positional.value,
        (&invocation.active, request.replace.clone()),
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

fn is_partial_subcommand(invocation: &Invocation) -> bool {
    invocation.args.len() == 1
        && !invocation.trailing_space
        && invocation.args[0] == invocation.active
}

#[cfg(test)]
#[path = "candidates_tests.rs"]
mod tests;
