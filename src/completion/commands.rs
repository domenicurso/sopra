use std::ops::Range;

use super::{CompletionItem, CompletionKind, CompletionSource};

pub(super) fn complete(query: &str, replace: Range<usize>) -> Vec<CompletionItem> {
    crate::syntax::catalog()
        .entries()
        .iter()
        .filter(|entry| query.is_empty() || fuzzy_contains(entry.name(), query))
        .map(|entry| {
            let kind = if entry.detail() == "builtin" {
                CompletionKind::Builtin
            } else {
                CompletionKind::Command
            };
            let mut item = CompletionItem::with_range(
                entry.name(),
                entry.name(),
                (entry.detail() == "builtin").then(|| "shell builtin".to_string()),
                kind,
                replace.clone(),
                CompletionSource::CommandIndex,
            );
            item.location = entry.location().map(ToOwned::to_owned);
            item
        })
        .take(512)
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
}
