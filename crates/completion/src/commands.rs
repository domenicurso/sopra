use std::ops::Range;

use super::command_catalog::{CommandEntry, catalog};
use super::{CompletionItem, CompletionKind, CompletionSource};

pub use super::command_discovery::command_available;
pub(crate) use super::command_discovery::resolve;

pub(super) fn complete(query: &str, replace: Range<usize>) -> Vec<CompletionItem> {
    catalog()
        .iter()
        .filter(|entry| entry.visible(query))
        .filter(|entry| query.is_empty() || fuzzy_contains(entry.name(), query))
        .map(|entry| completion_item(entry, replace.clone()))
        .collect()
}

fn completion_item(entry: &CommandEntry, replace: Range<usize>) -> CompletionItem {
    let description = match entry.detail() {
        "builtin" => Some("shell builtin".to_string()),
        "alias" => Some("shell alias".to_string()),
        "function" => Some("shell function".to_string()),
        _ => None,
    };
    let kind = match entry.detail() {
        "builtin" => CompletionKind::Builtin,
        "alias" => CompletionKind::Alias,
        "function" => CompletionKind::Function,
        _ => CompletionKind::Command,
    };
    let mut item = CompletionItem::with_range(
        entry.name(),
        entry.name(),
        description,
        kind,
        replace,
        CompletionSource::CommandIndex,
    );
    item.location = entry.location().map(ToOwned::to_owned);
    item
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
    use std::path::Path;

    use super::complete;

    #[test]
    fn command_completion_includes_builtins_and_prefix_matches() {
        let items = complete("ech", 0..3);
        assert!(items.iter().any(|item| item.display == "echo"));
    }

    #[test]
    fn path_commands_are_available_to_syntax_highlighting() {
        assert!(super::command_available("echo", Path::new("/")));
    }
}
