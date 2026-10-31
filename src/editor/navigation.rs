use unicode_segmentation::UnicodeSegmentation;

use super::{EditorState, RunResult};
use crate::actions::Action;
use crate::scene::MAX_OVERLAY_ITEMS;
use sopra_completion::ranking;

impl EditorState {
    pub(super) fn navigate(&mut self, key: Action) -> Option<RunResult> {
        if self.navigate_closed_history(key) {
            return None;
        }
        if self.accept_history(key) {
            return None;
        }
        match key {
            Action::Left => self.move_left(),
            Action::Right => self.move_right(),
            Action::WordLeft => self.move_word_left(),
            Action::WordRight => self.move_word_right(),
            Action::Home => self.cursor = 0,
            Action::End => self.cursor = self.buffer.len(),
            Action::Up if !self.move_selection(-1) && !self.move_vertical(-1) => {
                self.recall_history(-1);
            }
            Action::Down if !self.move_selection(1) && !self.move_vertical(1) => {
                self.recall_history(1);
            }
            _ => {}
        }
        None
    }

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
        let count = self.completions.items.len();
        if !self.completions.visible {
            return false;
        }
        if count == 0 {
            if self.completions.engine.is_some()
                && !self.completions.suspended()
                && !self.buffer.trim().is_empty()
                && !self.buffer.contains('\n')
            {
                self.completions.focus.set_armed(true);
            }
            return false;
        }
        let selected = self
            .completions
            .focus
            .selected()
            .or_else(|| self.history_preview_index());
        self.completions.focus.set_armed(true);
        self.completions.focus.select(Some(match selected {
            None => 0,
            Some(selected) if direction < 0 => selected.checked_sub(1).unwrap_or(count - 1),
            Some(selected) => (selected + 1) % count,
        }));
        self.remember_history_selection();
        self.update_suggestion_scroll();
        true
    }

    fn update_suggestion_scroll(&mut self) {
        let selected = self.completions.focus.selected().unwrap_or_default();
        let visible = self.completions.items.len().clamp(1, MAX_OVERLAY_ITEMS);
        let max_start = self.completions.items.len().saturating_sub(visible);
        if selected >= self.completions.scroll + visible.saturating_sub(1) {
            let desired = selected.saturating_sub(visible.saturating_sub(2));
            self.completions.scroll = self.completions.scroll.max(desired).min(max_start);
        } else if selected <= self.completions.scroll {
            self.completions.scroll = self.completions.scroll.min(selected.saturating_sub(1));
        };
    }

    pub(super) fn current_token_range(&self) -> (usize, usize) {
        ranking::token_range(&self.buffer, self.cursor)
    }
}

#[cfg(test)]
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
