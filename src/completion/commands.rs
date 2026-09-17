use std::ops::Range;

use super::{CompletionItem, CompletionKind, CompletionSource};

pub(super) fn complete(query: &str, replace: Range<usize>) -> Vec<CompletionItem> {
    crate::syntax::catalog()
        .entries()
        .iter()
        .filter(|entry| query.is_empty() || fuzzy_contains(entry.name(), query))
        .map(|entry| {
            let kind = match entry.detail() {
                "builtin" => CompletionKind::Builtin,
                "alias" => CompletionKind::Alias,
                "function" => CompletionKind::Function,
                _ => CompletionKind::Command,
            };
            let mut item = CompletionItem::with_range(
                entry.name(),
                entry.name(),
                match entry.detail() {
                    "builtin" => Some("shell builtin".to_string()),
                    "alias" => Some("shell alias".to_string()),
                    "function" => Some("shell function".to_string()),
                    _ => None,
                },
                kind,
                replace.clone(),
                CompletionSource::CommandIndex,
            );
            item.location = entry.location().map(ToOwned::to_owned);
            item
        })
        .collect()
}

fn fuzzy_contains(candidate: &str, query: &str) -> bool {
    let mut candidate = candidate.chars();
    query.chars().all(|query_char| {
        candidate
            .by_ref()
            .any(|candidate_char| candidate_char.eq_ignore_ascii_case(&query_char))
    })
}

#[cfg(test)]
mod tests {
    use super::complete;

    #[test]
    fn command_completion_includes_builtins_and_prefix_matches() {
        let items = complete("ech", 0..3);
        assert!(items.iter().any(|item| item.display == "echo"));
    }

    #[test]
    fn path_commands_keep_their_source_location() {
        let items = complete("sh", 0..2);
        assert!(items.iter().any(|item| item.location.is_some()));
    }
}
