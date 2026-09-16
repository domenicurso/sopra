use neo_frizbee::{Config, Matcher};

use super::{CompletionItem, CompletionKind, path};

pub(crate) fn broad_context(line: &str, cursor_chars: usize) -> (String, usize) {
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

pub(crate) fn token_range(line: &str, cursor: usize) -> (usize, usize) {
    let prefix = &line[..cursor];
    let start = prefix
        .char_indices()
        .rev()
        .find_map(|(index, character)| {
            is_token_boundary(character).then_some(index + character.len_utf8())
        })
        .unwrap_or(0);
    let suffix = &line[cursor..];
    let end = suffix
        .char_indices()
        .find_map(|(index, character)| is_token_boundary(character).then_some(cursor + index))
        .unwrap_or(line.len());
    (start, end)
}

fn is_token_boundary(character: char) -> bool {
    character.is_whitespace() || matches!(character, '|' | '&' | ';' | '(' | ')' | '<' | '>')
}

pub(crate) fn rank(items: &[CompletionItem], query: &str) -> Vec<CompletionItem> {
    let config = Config::default();
    let mut ranked = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let matched = if matches!(item.kind, CompletionKind::File | CompletionKind::Directory) {
            path::match_item(item, query, &config)
        } else {
            match_generic(item, query, &config)
        };
        let Some((score, indices)) = matched else {
            continue;
        };
        let mut item = item.clone();
        item.match_indices = indices;
        item.score = score as f32;
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
        .match_one_indices(&item.display, 0)
        .map(|value| (u32::from(value.score), value.indices))
}

pub(crate) fn byte_offset(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map_or(text.len(), |(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::{broad_context, rank, token_range};
    use crate::completion::{CompletionItem, CompletionKind};

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
    fn token_range_stops_at_shell_operators() {
        assert_eq!(token_range("echo|ec", 7), (5, 7));
    }
}
