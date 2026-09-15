use unicode_segmentation::UnicodeSegmentation;

use crate::input::Key;

use super::{EditorState, ExitReason, RunResult};

impl EditorState {
    pub(super) fn handle_key(&mut self, key: Key) -> Option<RunResult> {
        match key {
            Key::Character(character) => self.insert_character(character),
            Key::Enter => return Some(self.result(ExitReason::Accepted)),
            Key::Tab => self.apply_selected(),
            Key::Backspace => self.backspace(),
            Key::Delete => self.delete(),
            Key::Left => self.move_left(),
            Key::Right => self.move_right(),
            Key::Home => self.cursor = 0,
            Key::End => self.cursor = self.buffer.len(),
            Key::Up => self.move_selection(-1),
            Key::Down => self.move_selection(1),
            Key::Clear => self.toggle_overlay(),
            Key::Cancel => return Some(self.result(ExitReason::Interrupted)),
            Key::Escape => return Some(self.result(ExitReason::Cancelled)),
            Key::Eof => return Some(self.cancel_to_original()),
        }
        None
    }

    fn insert_character(&mut self, character: char) {
        self.buffer.insert(self.cursor, character);
        self.cursor += character.len_utf8();
        self.reset_selection();
        self.status = "editing".to_string();
    }

    fn toggle_overlay(&mut self) {
        self.overlay_visible = !self.overlay_visible;
        self.status = if self.overlay_visible {
            "overlay on"
        } else {
            "overlay off"
        }
        .to_string();
    }

    fn apply_selected(&mut self) {
        let items = self.visible_items();
        let Some(item) = items.get(self.selected).copied() else {
            return;
        };
        if item.label == "no matches" {
            return;
        }
        let (start, end) = self.current_token_range();
        self.buffer.replace_range(start..end, item.label);
        self.cursor = start + item.label.len();
        self.status = "inserted".to_string();
        self.reset_selection();
    }

    fn backspace(&mut self) {
        let Some(start) = previous_grapheme_start(&self.buffer, self.cursor) else {
            return;
        };
        self.buffer.replace_range(start..self.cursor, "");
        self.cursor = start;
        self.reset_selection();
    }

    fn delete(&mut self) {
        let Some(end) = next_grapheme_end(&self.buffer, self.cursor) else {
            return;
        };
        self.buffer.replace_range(self.cursor..end, "");
        self.reset_selection();
    }

    fn move_left(&mut self) {
        if let Some(start) = previous_grapheme_start(&self.buffer, self.cursor) {
            self.cursor = start;
        }
    }

    fn move_right(&mut self) {
        if let Some(end) = next_grapheme_end(&self.buffer, self.cursor) {
            self.cursor = end;
        }
    }

    fn move_selection(&mut self, direction: i8) {
        let count = self.visible_items().len();
        self.selected = if direction < 0 {
            self.selected
                .checked_sub(1)
                .unwrap_or(count.saturating_sub(1))
        } else {
            (self.selected + 1) % count.max(1)
        };
        self.status = format!("selected {}/{}", self.selected + 1, count);
    }

    fn reset_selection(&mut self) {
        self.selected = 0;
    }

    pub(super) fn current_token_range(&self) -> (usize, usize) {
        let before = &self.buffer[..self.cursor];
        let start = before
            .char_indices()
            .rev()
            .find_map(|(index, character)| {
                character
                    .is_whitespace()
                    .then_some(index + character.len_utf8())
            })
            .unwrap_or(0);
        let after = &self.buffer[self.cursor..];
        let end = after
            .char_indices()
            .find_map(|(index, character)| character.is_whitespace().then_some(self.cursor + index))
            .unwrap_or(self.buffer.len());
        (start, end)
    }

    pub(super) fn result(&self, reason: ExitReason) -> RunResult {
        RunResult {
            reason,
            buffer: self.buffer.clone(),
            cursor: self.buffer[..self.cursor].chars().count(),
        }
    }

    fn cancel_to_original(&mut self) -> RunResult {
        self.buffer = self.original_buffer.clone();
        self.cursor = self.original_cursor;
        self.result(ExitReason::Cancelled)
    }
}

pub(super) fn char_index_to_byte(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map_or(text.len(), |(index, _)| index)
}

fn previous_grapheme_start(text: &str, cursor: usize) -> Option<usize> {
    text[..cursor]
        .grapheme_indices(true)
        .next_back()
        .map(|(index, _)| index)
}

fn next_grapheme_end(text: &str, cursor: usize) -> Option<usize> {
    text[cursor..]
        .grapheme_indices(true)
        .nth(1)
        .map(|(index, _)| cursor + index)
        .or_else(|| (cursor < text.len()).then_some(text.len()))
}
