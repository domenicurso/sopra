mod collect;
mod collect_options;
mod usage;

use super::Document;
use crate::graph::{CommandNode, Synopsis};

pub(crate) fn parse(
    document: &Document,
    root: &mut CommandNode,
    path: &[String],
    discover_usage_commands: bool,
) {
    let usage_commands = discover_usage_commands
        && !path.is_empty()
        && document
            .usage
            .iter()
            .filter_map(|usage| usage::tokens(usage, &root.name, path))
            .any(|tokens| usage::has_command_form(&tokens));
    for usage in &document.usage {
        parse_usage(usage, root, path, usage_commands);
    }
}

fn parse_usage(
    usage: &str,
    root: &mut CommandNode,
    path: &[String],
    discover_usage_commands: bool,
) {
    let Some((tokens, synopsis)) = usage::parse(usage, &root.name, path) else {
        return;
    };
    root.synopsis.get_or_insert_with(|| synopsis.clone());
    if discover_usage_commands {
        collect::options_from_synopsis(&synopsis, root);
        add_usage_command(&tokens, root);
    } else {
        collect::from_synopsis(&synopsis, root, path.is_empty());
    }
}

fn add_usage_command(tokens: &[String], root: &mut CommandNode) {
    let Some(index) = usage::command_index(tokens) else {
        return;
    };
    let mut command = CommandNode::named(&tokens[index]);
    let mut child_index = 0;
    let child_tokens = &tokens[index + 1..];
    let expression = usage::parse_alternatives(child_tokens, &mut child_index, None);
    for alternative in expression {
        collect::from_synopsis(&Synopsis::Sequence(alternative), &mut command, false);
    }
    root.merge_subcommand(command);
}
