use sopra_completion::{CompletionItem, ranking};

use super::History;

impl History {
    // Only reorder equally strong completion matches; history never invents candidates.
    pub(in crate::editor) fn rank_contextual(
        &self,
        items: &mut [CompletionItem],
        buffer: &str,
        cursor: usize,
    ) {
        for tied in items.chunk_by_mut(|left, right| left.score.total_cmp(&right.score).is_eq()) {
            tied.sort_by_cached_key(|item| self.recent_use(item, buffer, cursor));
        }
    }

    fn recent_use(&self, item: &CompletionItem, buffer: &str, cursor: usize) -> usize {
        let start = if item.replace.start == 0 && item.replace.end == 0 {
            ranking::token_range(buffer, cursor).0
        } else {
            item.replace.start
        };
        let Some(before) = buffer.get(..start) else {
            return usize::MAX;
        };
        let prefix = format!("{before}{}{}", item.insert, item.suffix);
        self.entries
            .iter()
            .position(|entry| {
                entry.command.strip_prefix(&prefix).is_some_and(|rest| {
                    rest.is_empty()
                        || rest.starts_with(char::is_whitespace)
                        || prefix.ends_with('/')
                        || prefix.ends_with(char::is_whitespace)
                })
            })
            .unwrap_or(usize::MAX)
    }
}
