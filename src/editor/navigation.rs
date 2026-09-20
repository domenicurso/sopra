use unicode_segmentation::UnicodeSegmentation;

use super::EditorState;
use crate::scene::MAX_OVERLAY_ITEMS;
use sopra_completion::ranking;

impl EditorState {
    pub(super) fn delete(&mut self) {
        let Some(end) = next_grapheme_end(&self.buffer, self.cursor) else {
            return;
        };
        self.buffer.replace_range(self.cursor..end, "");
        self.reset_selection();
    }

    pub(super) fn move_left(&mut self) {
        if let Some(start) = previous_grapheme_start(&self.buffer, self.cursor) {
            self.cursor = start;
        }
    }

    pub(super) fn move_right(&mut self) {
        if let Some(end) = next_grapheme_end(&self.buffer, self.cursor) {
            self.cursor = end;
        }
    }

    pub(super) fn move_word_left(&mut self) {
        while let Some(start) = previous_grapheme_start(&self.buffer, self.cursor) {
            self.cursor = start;
            if !self.buffer[start..].chars().all(char::is_whitespace) {
                break;
            }
        }
        while let Some(start) = previous_grapheme_start(&self.buffer, self.cursor) {
            if self.buffer[start..self.cursor]
                .chars()
                .all(char::is_whitespace)
            {
                break;
            }
            self.cursor = start;
        }
    }

    pub(super) fn move_word_right(&mut self) {
        while let Some(end) = next_grapheme_end(&self.buffer, self.cursor) {
            if !self.buffer[self.cursor..end]
                .chars()
                .all(char::is_whitespace)
            {
                break;
            }
            self.cursor = end;
        }
        while let Some(end) = next_grapheme_end(&self.buffer, self.cursor) {
            self.cursor = end;
            if self.buffer[end..]
                .chars()
                .next()
                .is_none_or(char::is_whitespace)
            {
                break;
            }
        }
    }

    pub(super) fn move_selection(&mut self, direction: i8) -> bool {
        let count = self.suggestions.len();
        if count == 0 || !self.overlay_visible {
            return false;
        }
        let Some(selected) = self.selected else {
            return false;
        };
        self.selected = Some(if direction < 0 {
            selected.checked_sub(1).unwrap_or(count.saturating_sub(1))
        } else {
            (selected + 1) % count.max(1)
        });
        self.update_suggestion_scroll();
        true
    }

    fn update_suggestion_scroll(&mut self) {
        let selected = self.selected.unwrap_or_default();
        let visible = self.suggestions.len().clamp(1, MAX_OVERLAY_ITEMS);
        let max_start = self.suggestions.len().saturating_sub(visible);
        if selected >= self.suggestion_scroll + visible.saturating_sub(1) {
            let desired = selected.saturating_sub(visible.saturating_sub(2));
            self.suggestion_scroll = self.suggestion_scroll.max(desired).min(max_start);
        } else if selected <= self.suggestion_scroll {
            self.suggestion_scroll = self.suggestion_scroll.min(selected.saturating_sub(1));
        };
    }

    pub(super) fn current_token_range(&self) -> (usize, usize) {
        ranking::token_range(&self.buffer, self.cursor)
    }
}

pub(super) fn char_index_to_byte(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map_or(text.len(), |(index, _)| index)
}

pub(super) fn previous_grapheme_start(text: &str, cursor: usize) -> Option<usize> {
    text[..cursor]
        .grapheme_indices(true)
        .next_back()
        .map(|(index, _)| index)
}

pub(super) fn next_grapheme_end(text: &str, cursor: usize) -> Option<usize> {
    text[cursor..]
        .grapheme_indices(true)
        .nth(1)
        .map(|(index, _)| cursor + index)
        .or_else(|| (cursor < text.len()).then_some(text.len()))
}
