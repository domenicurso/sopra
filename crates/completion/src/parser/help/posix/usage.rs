use super::super::option;
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

fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for character in text.chars() {
        if character.is_whitespace() {
            push_token(&mut tokens, &mut current);
        } else if matches!(character, '[' | ']' | '(' | ')' | '|') {
            push_token(&mut tokens, &mut current);
            tokens.push(character.to_string());
        } else {
            current.push(character);
        }
    }
    push_token(&mut tokens, &mut current);
    tokens
}

fn push_token(tokens: &mut Vec<String>, current: &mut String) {
    if current.is_empty() {
        return;
    }
    if let Some(value) = current.strip_suffix("...") {
        if !value.is_empty() {
            tokens.push(value.to_string());
        }
        tokens.push("...".to_string());
    } else {
        tokens.push(std::mem::take(current));
    }
    current.clear();
}

pub(super) fn parse_alternatives(
    tokens: &[String],
    index: &mut usize,
    closing: Option<&str>,
) -> Vec<Vec<Synopsis>> {
    let mut alternatives = vec![Vec::new()];
    while *index < tokens.len() {
        let token = tokens[*index].as_str();
        if Some(token) == closing {
            *index += 1;
            break;
        }
        if token == "|" {
            alternatives.push(Vec::new());
            *index += 1;
            continue;
        }
        if token == "(" && is_note(tokens, *index) {
            skip_group(tokens, index, "(", ")");
            continue;
        }
        let Some(mut node) = parse_node(tokens, index) else {
            break;
        };
        if tokens.get(*index).is_some_and(|next| next == "...") {
            *index += 1;
            node = Synopsis::Repeat(Box::new(node));
        }
        alternatives
            .last_mut()
            .expect("alternative exists")
            .push(node);
    }
    alternatives
}

fn parse_node(tokens: &[String], index: &mut usize) -> Option<Synopsis> {
    let token = tokens.get(*index)?.as_str();
    let node = match token {
        "[" => {
            *index += 1;
            let inner = parse_alternatives(tokens, index, Some("]"));
            if inner.len() == 1 {
                Synopsis::Optional(inner.into_iter().next().unwrap_or_default())
            } else {
                Synopsis::Optional(vec![Synopsis::Choice(inner)])
            }
        }
        "(" => {
            *index += 1;
            let inner = parse_alternatives(tokens, index, Some(")"));
            if inner.len() == 1 {
                Synopsis::Sequence(inner.into_iter().next().unwrap_or_default())
            } else {
                Synopsis::Choice(inner)
            }
        }
        ")" | "]" => return None,
        _ => {
            *index += 1;
            Synopsis::Token(token.to_string())
        }
    };
    Some(node)
}

pub(super) fn is_note(tokens: &[String], start: usize) -> bool {
    let Some(end) = matching_end(tokens, start, "(", ")") else {
        return false;
    };
    let inner = &tokens[start + 1..end];
    if inner.is_empty() || inner.iter().any(|token| token == "|") {
        return false;
    }
    let first = inner[0].to_ascii_lowercase();
    first.starts_with('`')
        || inner
            .iter()
            .any(|token| token.contains('`') || token.contains("http"))
        || matches!(
            first.as_str(),
            "by" | "for" | "from" | "same" | "see" | "using" | "with"
        )
}

pub(super) fn skip_group(tokens: &[String], index: &mut usize, open: &str, close: &str) {
    let Some(end) = matching_end(tokens, *index, open, close) else {
        *index = tokens.len();
        return;
    };
    *index = end + 1;
}

fn matching_end(tokens: &[String], start: usize, open: &str, close: &str) -> Option<usize> {
    let mut depth = 0;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        if token == open {
            depth += 1;
        } else if token == close {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}
