#[cfg(test)]
mod tests;

use std::{collections::BTreeSet, ops::Range};

use super::{CompletionItem, CompletionKind, CompletionSource, Request};

pub(super) fn complete(
    request: &Request,
    replace: Range<usize>,
    token: &str,
) -> Option<Vec<CompletionItem>> {
    let expansion = expansion_context(request, &replace, token);
    let declaration = is_declaration_context(&request.line[..replace.start]);
    if expansion.is_none() && !declaration {
        return None;
    }
    let (replace, prefix, query) =
        expansion.unwrap_or_else(|| (replace, String::new(), String::new()));
    let expansion = !prefix.is_empty();
    let names = names();
    let arrays = arrays();
    let items = names
        .into_iter()
        .filter(|name| query.is_empty() || fuzzy_contains(name, &query))
        .map(|name| {
            let is_array = arrays.contains(name.as_str());
            let insert = if expansion {
                format!("{prefix}{name}")
            } else {
                name.clone()
            };
            let mut item = CompletionItem::with_range(
                insert.clone(),
                insert,
                Some(
                    if is_array {
                        "shell array"
                    } else {
                        "shell variable"
                    }
                    .to_string(),
                ),
                if is_array {
                    CompletionKind::Array
                } else {
                    CompletionKind::Variable
                },
                replace.clone(),
                CompletionSource::ShellContext,
            );
            if declaration && !expansion {
                item = if is_array {
                    item.with_suffix("=()", name.len() + 1)
                } else {
                    item.with_suffix("=\"\"", name.len() + 2)
                };
            }
            item
        })
        .collect();
    Some(items)
}

fn expansion_context(
    request: &Request,
    replace: &Range<usize>,
    token: &str,
) -> Option<(Range<usize>, String, String)> {
    let cursor = crate::completion::ranking::byte_offset(&request.line, request.cursor);
    if crate::syntax::quote_context(&request.line, cursor) == Some('\'') {
        return None;
    }
    let dollar = token
        .match_indices('$')
        .rev()
        .find_map(|(index, _)| (!is_escaped(&token[..index])).then_some(index))?;
    let fragment = &token[dollar + 1..];
    let absolute = replace.start + dollar;
    if let Some(rest) = fragment.strip_prefix('{') {
        let (query, has_close) = rest
            .split_once('}')
            .map_or((rest, false), |(query, _)| (query, true));
        let query_len = query.len();
        let remainder = if has_close {
            &rest[query_len + 1..]
        } else {
            &rest[query_len..]
        };
        if !remainder.is_empty() {
            return None;
        }
        return Some((
            absolute..absolute + 2 + query_len,
            "${".to_string(),
            query.to_string(),
        ));
    }
    let query_len = fragment
        .char_indices()
        .take_while(|(_, character)| valid_name_char(*character))
        .last()
        .map_or(0, |(index, character)| index + character.len_utf8());
    if query_len != fragment.len() {
        return None;
    }
    Some((
        absolute..absolute + 1 + query_len,
        "$".to_string(),
        fragment[..query_len].to_string(),
    ))
}

fn is_escaped(value: &str) -> bool {
    value
        .chars()
        .rev()
        .take_while(|character| *character == '\\')
        .count()
        % 2
        == 1
}

fn valid_name_char(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}

fn names() -> BTreeSet<String> {
    let mut names = std::env::vars_os()
        .filter_map(|(name, _)| name.into_string().ok())
        .filter(|name| valid_name(name))
        .collect::<BTreeSet<_>>();
    if let Ok(raw) = std::env::var("SOPRA_VARIABLES") {
        names.extend(
            raw.lines()
                .filter(|name| valid_name(name))
                .map(str::to_string),
        );
    }
    names
}

fn arrays() -> BTreeSet<String> {
    std::env::var("SOPRA_ARRAYS")
        .unwrap_or_default()
        .lines()
        .filter(|name| valid_name(name))
        .map(str::to_string)
        .collect()
}

fn is_declaration_context(prefix: &str) -> bool {
    let words = prefix.split_whitespace().collect::<Vec<_>>();
    words.last().is_some_and(|word| {
        matches!(
            *word,
            "export" | "local" | "typeset" | "declare" | "readonly"
        ) || *word == "-a"
            && words
                .iter()
                .any(|candidate| matches!(*candidate, "local" | "typeset" | "declare" | "readonly"))
    })
}

fn valid_name(name: &str) -> bool {
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn fuzzy_contains(candidate: &str, query: &str) -> bool {
    let mut candidate = candidate.chars();
    query.chars().all(|query_char| {
        candidate
            .by_ref()
            .any(|candidate_char| candidate_char.eq_ignore_ascii_case(&query_char))
    })
}
