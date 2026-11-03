use super::super::{layout::Row, option};
use super::{
    arguments::append_arguments,
    paths::{command_name, command_path},
    rows::{aliases, description_is_link, description_text, signature},
};
use crate::graph::CommandNode;

pub(super) fn parse(row: &Row, root: &CommandNode, path: &[String]) -> Option<CommandNode> {
    let signature = signature(row);
    let description_text = description_text(row);
    let described = !description_text.trim().is_empty();
    if description_is_link(&description_text) {
        return None;
    }
    let parts = signature.split(',').map(str::trim).collect::<Vec<_>>();
    let (command_names, argument_start) =
        command_path(parts.first().copied()?, root.name.as_str(), path, described)?;
    let mut command = CommandNode::named(&command_names[0]);
    command.aliases = parts
        .iter()
        .skip(1)
        .filter_map(|part| command_name(part, "", &[], described))
        .collect();
    command.aliases.extend(aliases(&description_text));
    command.aliases.retain(|alias| alias != &command_names[0]);
    append_command_path(&mut command, &command_names[1..]);
    let leaf = command_leaf(&mut command, &command_names[1..]);
    leaf.description = option::clean_description(&description_text);
    let tokens = signature
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    append_arguments(leaf, &tokens, argument_start);
    append_children(leaf, &row.children);
    Some(command)
}

fn append_command_path(command: &mut CommandNode, names: &[String]) {
    let Some(name) = names.first() else {
        return;
    };
    command.subcommands.push(CommandNode::named(name));
    append_command_path(
        command.subcommands.last_mut().expect("just appended"),
        &names[1..],
    );
}

fn command_leaf<'a>(command: &'a mut CommandNode, names: &[String]) -> &'a mut CommandNode {
    let Some(name) = names.first() else {
        return command;
    };
    let child = command
        .subcommands
        .iter_mut()
        .find(|child| child.name == *name)
        .expect("command path was appended");
    command_leaf(child, &names[1..])
}

fn append_children(command: &mut CommandNode, children: &[Row]) {
    for child in children {
        let signature = signature(child);
        let first = signature.split_whitespace().next().unwrap_or_default();
        if option::is_option_token(first) {
            if let Some(parsed) = option::option_row(&signature, &description_text(child)) {
                command.merge_option(parsed);
            }
            continue;
        }
        if enrich_positional(command, child) {
            continue;
        }
        if let Some(nested) = child_command(child) {
            command.merge_subcommand(nested);
        }
    }
}

fn child_command(row: &Row) -> Option<CommandNode> {
    let signature = signature(row);
    let name = command_name(&signature, "", &[], true)?;
    let mut command = CommandNode::named(name);
    command.description = option::clean_description(&description_text(row));
    let tokens = signature
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    append_arguments(&mut command, &tokens, 1);
    append_children(&mut command, &row.children);
    Some(command)
}

fn enrich_positional(command: &mut CommandNode, row: &Row) -> bool {
    let signature = signature(row);
    let mut tokens = signature.split_whitespace();
    let Some(name) = tokens.next() else {
        return false;
    };
    if tokens.next().is_some() {
        return false;
    }
    let Some(positional) = command
        .positionals
        .iter_mut()
        .find(|value| value.name == name)
    else {
        return false;
    };
    let choices = option::choices_from_description(&description_text(row));
    if choices.is_empty() {
        return false;
    }
    for choice in choices {
        if !positional.value.choices.contains(&choice) {
            positional.value.choices.push(choice);
        }
    }
    true
}

pub(super) fn positional_command(row: &Row, choices: &[String]) -> Option<CommandNode> {
    let signature = signature(row);
    let tokens = signature.split_whitespace().collect::<Vec<_>>();
    let name = tokens.first()?.trim_matches(['<', '>', '[', ']']);
    if !choices.iter().any(|choice| choice == name) {
        return None;
    }
    let mut command = CommandNode::named(name);
    command.description = option::clean_description(&description_text(row));
    let tokens = tokens
        .into_iter()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    append_arguments(&mut command, &tokens, 1);
    append_children(&mut command, &row.children);
    Some(command)
}
