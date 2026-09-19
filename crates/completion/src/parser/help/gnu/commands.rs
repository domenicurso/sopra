use super::super::{layout::Row, option};
use crate::graph::{CommandNode, PositionalSpec, ValueSpec};

pub(super) fn parse(row: &Row, root: &CommandNode, path: &[String]) -> Option<CommandNode> {
    let signature = signature(row);
    let description_text = description_text(row);
    let described = !description_text.trim().is_empty();
    if description_is_link(&description_text) {
        return None;
    }
    let parts = signature.split(',').map(str::trim).collect::<Vec<_>>();
    let first = command_name(parts.first().copied()?, root.name.as_str(), path, described)?;
    let mut command = CommandNode::named(&first);
    command.aliases = parts
        .iter()
        .skip(1)
        .filter_map(|part| command_name(part, "", &[], described))
        .collect();
    command.aliases.extend(aliases(&description_text));
    command.aliases.retain(|alias| alias != &first);
    command.description = option::clean_description(&description_text);
    let tokens = signature
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    append_arguments(&mut command, &tokens, 1);
    append_children(&mut command, &row.children);
    Some(command)
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

fn append_arguments(command: &mut CommandNode, tokens: &[String], start: usize) {
    let mut index = start;
    while index < tokens.len() {
        let (token, next) = argument_token(tokens, index);
        if option::placeholder(&token) {
            let name = clean_placeholder(&token);
            if !name.is_empty() {
                command.positionals.push(PositionalSpec {
                    value: ValueSpec::named(&name),
                    name,
                    description: None,
                    optional: token.starts_with('['),
                    repeatable: token.contains("...") || token.contains(".."),
                });
            }
        }
        index = next;
    }
}

fn argument_token(tokens: &[String], start: usize) -> (String, usize) {
    let mut token = tokens[start].clone();
    let close = if token.starts_with('[') {
        Some(']')
    } else if token.starts_with('<') {
        Some('>')
    } else {
        None
    };
    let mut index = start + 1;
    while let Some(close) = close {
        if token.ends_with(close) || index >= tokens.len() {
            break;
        }
        token.push(' ');
        token.push_str(&tokens[index]);
        index += 1;
    }
    (token, index)
}

fn signature(row: &Row) -> String {
    let Some(first) = row.columns.first() else {
        return row.left.clone();
    };
    let mut fields = vec![first.clone()];
    for column in row.columns.iter().skip(1) {
        if argument_column(column) {
            fields.push(column.clone());
        } else {
            break;
        }
    }
    fields.join(" ")
}

fn argument_column(value: &str) -> bool {
    let first = value.split_whitespace().next().unwrap_or_default();
    first.starts_with(['<', '[', '{', '-']) || first == "..."
}

fn description_text(row: &Row) -> String {
    if row.columns.len() > 2
        && row
            .columns
            .last()
            .is_some_and(|value| metadata_column(value))
    {
        return row.columns[1..].join(" ");
    }
    row.description.clone()
}

fn metadata_column(value: &str) -> bool {
    value.starts_with('[') && value.ends_with(']')
}

fn aliases(description: &str) -> Vec<String> {
    let lower = description.to_ascii_lowercase();
    let Some(start) = lower.find("[aliases:") else {
        return Vec::new();
    };
    let start = start + "[aliases:".len();
    let end = description[start..]
        .find(']')
        .map_or(description.len(), |offset| start + offset);
    description[start..end]
        .split([',', '|', ' '])
        .map(str::trim)
        .filter(|alias| !alias.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub(super) fn positional(row: &Row) -> Option<PositionalSpec> {
    let signature = signature(row);
    let token = signature.split_whitespace().next()?;
    if token.starts_with('-') || (token.starts_with('{') && token.ends_with('}')) {
        return None;
    }
    let name = clean_placeholder(token);
    Some(PositionalSpec {
        value: ValueSpec::named(&name),
        name,
        description: option::clean_description(&description_text(row)),
        optional: token.starts_with('['),
        repeatable: token.ends_with("...") || token.contains(".."),
    })
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

pub(super) fn choice_names(left: &str) -> Vec<String> {
    let Some(start) = left.find('{') else {
        return Vec::new();
    };
    let Some(end) = left[start + 1..].find('}') else {
        return Vec::new();
    };
    left[start + 1..start + 1 + end]
        .split([',', '|'])
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn command_name(value: &str, root: &str, path: &[String], allow_colon: bool) -> Option<String> {
    let tokens = value.split_whitespace().collect::<Vec<_>>();
    let start = command_prefix(&tokens, root, path)?;
    tokens
        .get(start..)?
        .iter()
        .find(|token| !option::placeholder(token) && !token.starts_with('-'))
        .map(|token| {
            token
                .trim_matches([',', '.', '<', '>', '[', ']', ':'])
                .to_string()
        })
        .filter(|name| {
            !name.is_empty()
                && name != "..."
                && name
                    .chars()
                    .any(|character| character.is_ascii_alphanumeric())
                && !name.ends_with(':')
                && !manual_reference(name)
                && (allow_colon || !value.trim().ends_with([':', '.']))
        })
}

fn command_prefix(tokens: &[&str], root: &str, path: &[String]) -> Option<usize> {
    if tokens.first().is_none_or(|token| *token != root) {
        return Some(0);
    }
    let expected = std::iter::once(root).chain(path.iter().map(String::as_str));
    let matches = tokens
        .iter()
        .zip(expected)
        .all(|(actual, expected)| *actual == expected)
        && tokens.len() > path.len();
    matches.then_some(path.len() + 1)
}

fn description_is_link(value: &str) -> bool {
    value
        .split_whitespace()
        .any(|token| token.starts_with("http://") || token.starts_with("https://"))
}

fn manual_reference(name: &str) -> bool {
    let Some(start) = name.rfind('(') else {
        return false;
    };
    let Some(section) = name.get(start + 1..name.len().saturating_sub(1)) else {
        return false;
    };
    name.ends_with(')')
        && !name[..start].is_empty()
        && !section.is_empty()
        && section
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
}

fn clean_placeholder(value: &str) -> String {
    let value = value.trim();
    if value.starts_with(['<', '[']) {
        value
            .trim_matches(['<', '>', '[', ']'])
            .trim()
            .trim_end_matches("...")
            .trim()
            .to_string()
    } else {
        value.trim_end_matches("...").trim().to_string()
    }
}
