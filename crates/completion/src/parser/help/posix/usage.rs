use super::super::option;
pub(super) use super::expression::parse_alternatives;
use super::synopsis_tokens::tokenize;
use crate::graph::Synopsis;

pub(super) fn parse(text: &str, program: &str, path: &[String]) -> Option<(Vec<String>, Synopsis)> {
    let tokens = tokenize(text);
    let start = prefix_length(&tokens, program, path)?;
    let tokens = tokens.get(start..)?.to_vec();
    let mut index = 0;
    let alternatives = parse_alternatives(&tokens, &mut index, None);
    let synopsis = if alternatives.len() == 1 {
        Synopsis::Sequence(alternatives.into_iter().next().unwrap_or_default())
    } else {
        Synopsis::Choice(alternatives)
    };
    Some((tokens, synopsis))
}

pub(super) fn tokens(text: &str, program: &str, path: &[String]) -> Option<Vec<String>> {
    let tokens = tokenize(text);
    let start = prefix_length(&tokens, program, path)?;
    Some(tokens.get(start..)?.to_vec())
}

pub(super) fn has_command_form(tokens: &[String]) -> bool {
    let mut literals = 0;
    let mut depth: usize = 0;
    let mut value_pending = false;
    for token in tokens {
        if value_pending {
            value_pending = false;
            continue;
        }
        match token.as_str() {
            "[" | "(" => depth += 1,
            "]" | ")" => depth = depth.saturating_sub(1),
            _ if depth == 0 && option::is_option_token(token) => value_pending = true,
            _ if depth == 0 && literal_command(token) => {
                literals += 1;
                if literals > 1 {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

pub(super) fn command_index(tokens: &[String]) -> Option<usize> {
    let mut depth = 0_usize;
    let mut value_pending = false;
    tokens.iter().enumerate().find_map(|(index, token)| {
        if value_pending {
            value_pending = false;
            return None;
        }
        match token.as_str() {
            "[" | "(" => depth += 1,
            "]" | ")" => depth = depth.saturating_sub(1),
            _ if depth == 0 && option::is_option_token(token) => value_pending = true,
            _ if depth == 0 && literal_command(token) => return Some(index),
            _ => {}
        }
        None
    })
}

fn prefix_length(tokens: &[String], program: &str, path: &[String]) -> Option<usize> {
    let expected = std::iter::once(program).chain(path.iter().map(String::as_str));
    let mut length = 0;
    for expected in expected {
        if tokens.get(length).is_none_or(|token| token != expected) {
            return None;
        }
        length += 1;
    }
    Some(length)
}

fn literal_command(token: &str) -> bool {
    !token.is_empty()
        && token != "..."
        && !token.ends_with("...")
        && !token.starts_with(['-', '<', '{', '='])
        && token
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "_:.@/".contains(character))
}
