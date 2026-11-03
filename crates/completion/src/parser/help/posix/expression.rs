use crate::graph::Synopsis;

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
    expand_assignment_choices(&mut alternatives);
    alternatives
}

fn expand_assignment_choices(alternatives: &mut [Vec<Synopsis>]) {
    let Some(prefix) = alternatives
        .iter()
        .find_map(|alternative| assignment_prefix(alternative))
    else {
        return;
    };
    for alternative in alternatives {
        let Some(Synopsis::Token(token)) = alternative.first_mut() else {
            continue;
        };
        if !token.contains('=') && !token.starts_with('-') {
            *token = format!("{prefix}{token}");
        }
    }
}

fn assignment_prefix(alternative: &[Synopsis]) -> Option<String> {
    let Synopsis::Token(token) = alternative.first()? else {
        return None;
    };
    token
        .split_once('=')
        .map(|(prefix, _)| format!("{prefix}="))
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
