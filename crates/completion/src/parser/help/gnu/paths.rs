use super::super::option;
use super::arguments;

pub(super) fn command_path(
    value: &str,
    root: &str,
    path: &[String],
    allow_colon: bool,
) -> Option<(Vec<String>, usize)> {
    let tokens = value.split_whitespace().collect::<Vec<_>>();
    let start = command_prefix(&tokens, root, path)?;
    let mut names = Vec::new();
    for (index, token) in tokens.iter().enumerate().skip(start) {
        if arguments::is_argument(token, tokens.get(index + 1).copied())
            || token.starts_with('-')
            || *token == "..."
        {
            break;
        }
        let name = token.trim_matches([',', '.', '<', '>', '[', ']', ':']);
        if name.is_empty() || manual_reference(name) || name.ends_with(':') {
            break;
        }
        if !allow_colon && value.trim().ends_with([':', '.']) {
            break;
        }
        names.push(name.to_string());
    }
    let end = start + names.len();
    (!names.is_empty()).then_some((names, end))
}

pub(super) fn command_name(
    value: &str,
    root: &str,
    path: &[String],
    allow_colon: bool,
) -> Option<String> {
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

pub(super) fn command_prefix(tokens: &[&str], root: &str, path: &[String]) -> Option<usize> {
    if tokens.first().is_none_or(|token| *token != root) {
        let matches_path = tokens
            .iter()
            .zip(path.iter().map(String::as_str))
            .all(|(actual, expected)| *actual == expected)
            && tokens.len() > path.len();
        if matches_path {
            return Some(path.len());
        }
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

pub(super) fn manual_reference(name: &str) -> bool {
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
