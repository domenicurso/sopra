use neo_frizbee::{Config, Matcher};

use super::{CompletionItem, CompletionKind, path};

pub(crate) fn broad_context(line: &str, cursor_chars: usize) -> (String, usize) {
    let cursor = byte_offset(line, cursor_chars);
    let (start, end) = token_range(line, cursor);
    let provider_line = format!("{}{}", &line[..start], &line[end..]);
    (provider_line, line[..start].chars().count())
}

pub(crate) fn token_range(line: &str, cursor: usize) -> (usize, usize) {
    let prefix = &line[..cursor];
    let start = prefix
        .char_indices()
        .rev()
        .find_map(|(index, character)| {
            character
                .is_whitespace()
                .then_some(index + character.len_utf8())
        })
        .unwrap_or(0);
    let suffix = &line[cursor..];
    let end = suffix
        .char_indices()
        .find_map(|(index, character)| character.is_whitespace().then_some(cursor + index))
        .unwrap_or(line.len());
    (start, end)
}

pub(crate) fn rank(items: &[CompletionItem], query: &str) -> Vec<CompletionItem> {
    let config = Config::default();
    let mut ranked = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let matched = if item.kind == CompletionKind::Generic {
            match_generic(item, query, &config)
        } else {
            path::match_item(item, query, &config)
        };
        let Some((score, indices)) = matched else {
            continue;
        };
        let mut item = item.clone();
        item.match_indices = indices;
        ranked.push((score, index, item));
    }
    ranked.sort_by(|left, right| right.0.cmp(&left.0).then(left.1.cmp(&right.1)));
    let mut ranked = ranked
        .into_iter()
        .take(256)
        .map(|(_, _, item)| item)
        .collect::<Vec<_>>();
    path::compact_labels(&mut ranked, query);
    ranked
}

fn match_generic(item: &CompletionItem, query: &str, config: &Config) -> Option<(u32, Vec<usize>)> {
    if query.is_empty() {
        return Some((0, Vec::new()));
    }
    Matcher::new(query, config)
        .match_one_indices(&item.label, 0)
        .map(|value| (u32::from(value.score), value.indices))
}

pub(crate) fn byte_offset(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map_or(text.len(), |(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::{broad_context, rank};
    use crate::completion::{CompletionItem, CompletionKind};

    #[test]
    fn broad_context_removes_only_the_active_token() {
        let (line, cursor) = broad_context("echo 💡", 6);
        assert_eq!(line, "echo ");
        assert_eq!(cursor, 5);
    }

    #[test]
    fn generic_results_use_fuzzy_scores_and_highlights() {
        let items = vec![
            CompletionItem::new("git status", "", "git status", CompletionKind::Generic),
            CompletionItem::new("cargo test", "", "cargo test", CompletionKind::Generic),
        ];
        let ranked = rank(&items, "gs");
        assert_eq!(ranked[0].label, "git status");
        assert!(!ranked[0].match_indices.is_empty());
    }
}
