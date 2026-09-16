use super::{CompletionItem, CompletionKind};

pub(super) fn complete(query: &str) -> Vec<CompletionItem> {
    if query.is_empty() {
        return Vec::new();
    }
    crate::syntax::catalog()
        .entries()
        .iter()
        .map(|entry| {
            CompletionItem::new(
                entry.name(),
                entry.detail(),
                entry.name(),
                CompletionKind::Generic,
            )
        })
        .filter(|item| fuzzy_contains(&item.label, query))
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
        let items = complete("ech");
        assert!(items.iter().any(|item| item.label == "echo"));
    }
}
