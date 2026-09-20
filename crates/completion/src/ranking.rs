use std::cmp::Ordering;

use neo_frizbee::{Config, Matcher};
use unicode_width::UnicodeWidthStr;

use super::{CompletionItem, CompletionKind, path};

pub use super::token::token_range;

pub fn broad_context(line: &str, cursor_chars: usize) -> (String, usize) {
    let cursor = byte_offset(line, cursor_chars);
    let (start, end) = token_range(line, cursor);
    let token = &line[start..cursor];
    let context_token = context_token(token);
    let context_line = format!("{}{}{}", &line[..start], context_token, &line[end..]);
    let context_cursor = line[..start].chars().count() + context_token.chars().count();
    (context_line, context_cursor)
}

fn context_token(token: &str) -> &str {
    let path_start = token
        .find('=')
        .filter(|_| token.starts_with('-'))
        .map_or(0, |index| index + 1);
    let path = &token[path_start..];
    if let Some(slash) = path.rfind('/') {
        return &token[..path_start + slash + 1];
    }
    if token.starts_with('-') {
        return if token.starts_with("--") {
            &token[..2]
        } else {
            &token[..1]
        };
    }
    ""
}

pub fn rank(items: &[CompletionItem], query: &str) -> Vec<CompletionItem> {
    rank_with(items, |_| query.to_string(), query)
}

pub fn rank_for_buffer(
    items: &[CompletionItem],
    buffer: &str,
    cursor: usize,
) -> Vec<CompletionItem> {
    let cursor = cursor.min(buffer.len());
    let (token_start, _) = token_range(buffer, cursor);
    let query = buffer.get(token_start..cursor).unwrap_or_default();
    rank_with(
        items,
        |item| replacement_query(buffer, cursor, item).to_string(),
        query,
    )
}

pub fn replacement_query<'a>(buffer: &'a str, cursor: usize, item: &CompletionItem) -> &'a str {
    let cursor = cursor.min(buffer.len());
    let (token_start, _) = token_range(buffer, cursor);
    let start = item.replace.start.clamp(token_start, cursor);
    buffer.get(start..cursor).unwrap_or_default()
}

fn rank_with(
    items: &[CompletionItem],
    mut query_for: impl FnMut(&CompletionItem) -> String,
    compact_query: &str,
) -> Vec<CompletionItem> {
    let config = Config::default();
    let mut ranked = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let query = query_for(item);
        let matched = if matches!(item.kind, CompletionKind::File | CompletionKind::Directory) {
            path::match_item(item, &query, &config)
        } else {
            match_generic(item, &query, &config)
        };
        let Some((score, indices)) = matched else {
            continue;
        };
        let mut item = item.clone();
        item.match_indices = indices;
        item.score = score as f32;
        ranked.push((score, index, item));
    }
    ranked.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| label_order(&left.2.display, &right.2.display))
            .then(left.1.cmp(&right.1))
    });
    ranked.dedup_by(|left, right| {
        left.2.display == right.2.display
            && left.2.insert == right.2.insert
            && left.2.kind == right.2.kind
    });
    let mut ranked = ranked
        .into_iter()
        .map(|(_, _, item)| item)
        .collect::<Vec<_>>();
    path::compact_labels(&mut ranked, compact_query);
    ranked
}

fn label_order(left: &str, right: &str) -> Ordering {
    left.to_ascii_lowercase()
        .cmp(&right.to_ascii_lowercase())
        .then_with(|| left.cmp(right))
}

pub fn completion_token_width(items: &[CompletionItem], query: &str) -> usize {
    let path = &query[path::path_value_offset(query)..];
    if !path.contains('/') {
        return path.rsplit('/').next().map_or(0, UnicodeWidthStr::width);
    }
    let components = path::path_components(path);
    if components.is_empty() {
        return 0;
    }
    let references = items
        .iter()
        .filter(|item| item.kind != super::CompletionKind::Generic)
        .collect::<Vec<_>>();
    if references.is_empty() {
        return UnicodeWidthStr::width(path);
    }
    let resolved = path::resolved_path_prefix(&references, path);
    if path.ends_with('/') && resolved == components.len() {
        return 0;
    }
    let first_unresolved = resolved.min(components.len().saturating_sub(1));
    let start = components[first_unresolved].1;
    let end = path.len().saturating_sub(usize::from(path.ends_with('/')));
    UnicodeWidthStr::width(&path[start..end]) + usize::from(path.ends_with('/'))
}

fn match_generic(item: &CompletionItem, query: &str, config: &Config) -> Option<(u32, Vec<usize>)> {
    if query.is_empty() {
        return Some((0, Vec::new()));
    }
    Matcher::new(query, config)
        .match_one_indices(&item.display, 0)
        .map(|value| (u32::from(value.score), value.indices))
}

pub fn byte_offset(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map_or(text.len(), |(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::{broad_context, rank, token_range};
    use crate::{CompletionItem, CompletionKind, CompletionSource};

    #[test]
    fn broad_context_removes_only_the_active_token() {
        let (line, cursor) = broad_context("echo 💡", 6);
        assert_eq!(line, "echo ");
        assert_eq!(cursor, 5);
    }

    #[test]
    fn broad_context_preserves_option_and_path_prefixes() {
        assert_eq!(broad_context("git --", 6), ("git --".to_string(), 6));
        assert_eq!(broad_context("cd ./sr", 7), ("cd ./".to_string(), 5));
    }

    #[test]
    fn generic_results_use_fuzzy_scores_and_highlights() {
        let items = vec![
            CompletionItem::new("git status", "", "git status", CompletionKind::Generic),
            CompletionItem::new("cargo test", "", "cargo test", CompletionKind::Generic),
        ];
        let ranked = rank(&items, "gs");
        assert_eq!(ranked[0].display, "git status");
        assert!(!ranked[0].match_indices.is_empty());
    }

    #[test]
    fn ranking_keeps_large_completion_sets() {
        let items = (0..300)
            .map(|index| {
                CompletionItem::new(
                    format!("entry-{index}"),
                    "",
                    format!("entry-{index}"),
                    CompletionKind::Generic,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(rank(&items, "").len(), 300);
    }

    #[test]
    fn empty_queries_use_stable_alphabetical_order_and_deduplicate() {
        let items = vec![
            CompletionItem::new("zeta", "", "zeta", CompletionKind::Option),
            CompletionItem::new("Alpha", "", "Alpha", CompletionKind::Option),
            CompletionItem::new("alpha", "", "alpha", CompletionKind::Option),
            CompletionItem::new("zeta", "", "zeta", CompletionKind::Option),
        ];
        let ranked = rank(&items, "");
        assert_eq!(
            ranked
                .iter()
                .map(|item| item.display.as_str())
                .collect::<Vec<_>>(),
            ["Alpha", "alpha", "zeta"]
        );
    }

    #[test]
    fn option_queries_keep_the_option_prefix() {
        let items = vec![CompletionItem::new(
            "--version",
            "show the version",
            "--version",
            CompletionKind::Option,
        )];
        let ranked = rank(&items, "--vrsn");
        assert_eq!(ranked[0].display, "--version");
    }

    #[test]
    fn ranking_uses_the_replacement_suffix_for_attached_values() {
        let item = CompletionItem::with_range(
            "private",
            "private",
            None,
            CompletionKind::Value,
            12..12,
            CompletionSource::CommandIndex,
        );
        let ranked = super::rank_for_buffer(&[item], "tool status=", 12);

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].display, "private");
        assert_eq!(super::replacement_query("tool status=", 12, &ranked[0]), "");
    }

    #[test]
    fn token_range_stops_at_shell_operators() {
        assert_eq!(token_range("echo|ec", 7), (5, 7));
    }
}
