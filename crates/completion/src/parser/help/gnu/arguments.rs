use super::super::{layout::Row, option};
use super::rows::{description_text, signature};
use crate::graph::{CommandNode, PositionalSpec, ValueSpec};

pub(super) fn append_arguments(command: &mut CommandNode, tokens: &[String], start: usize) {
    let mut index = start;
    while index < tokens.len() {
        let (token, next) = argument_token(tokens, index);
        if is_argument(&token, tokens.get(next).map(String::as_str)) {
            let name = clean_placeholder(&token);
            if !name.is_empty() {
                command.positionals.push(PositionalSpec {
                    value: ValueSpec::named(&name),
                    name,
                    description: None,
                    optional: token.starts_with('['),
                    repeatable: token.contains("..")
                        || tokens.get(next).is_some_and(|token| token == "..."),
                });
            }
        }
        index = next;
    }
}

pub(super) fn argument_token(tokens: &[String], start: usize) -> (String, usize) {
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

pub(super) fn clean_placeholder(value: &str) -> String {
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

pub(super) fn is_argument(token: &str, next: Option<&str>) -> bool {
    option::placeholder(token)
        || token.contains('=')
        || token.ends_with("...")
        || next == Some("...")
        || (token.chars().any(char::is_alphabetic)
            && token
                .chars()
                .filter(|character| character.is_alphabetic())
                .all(char::is_uppercase))
}
