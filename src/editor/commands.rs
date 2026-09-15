use super::{EditorState, ExitReason, RunResult, navigation::previous_grapheme_start};

impl EditorState {
    pub(super) fn insert_character(&mut self, character: char) {
        self.buffer.insert(self.cursor, character);
        self.cursor += character.len_utf8();
        self.reset_selection();
    }

    pub(super) fn toggle_overlay(&mut self) {
        self.overlay_visible = !self.overlay_visible;
    }

    pub(super) fn apply_selected(&mut self) -> bool {
        let Some(item) = self.suggestions.get(self.selected).cloned() else {
            return false;
        };
        let (start, end) = self.current_token_range();
        self.buffer.replace_range(start..end, &item.replacement);
        self.cursor = start + item.replacement.len();
        self.reset_selection();
        true
    }

    pub(super) fn backspace(&mut self) {
        let Some(start) = previous_grapheme_start(&self.buffer, self.cursor) else {
            return;
        };
        self.buffer.replace_range(start..self.cursor, "");
        self.cursor = start;
        self.reset_selection();
    }

    pub(super) fn delete_word_back(&mut self) {
        let end = self.cursor;
        let mut start = end;
        while let Some(previous) = previous_grapheme_start(&self.buffer, start) {
            if !self.buffer[previous..start]
                .chars()
                .all(char::is_whitespace)
            {
                break;
            }
            start = previous;
        }
        while let Some(previous) = previous_grapheme_start(&self.buffer, start) {
            if self.buffer[previous..start]
                .chars()
                .all(char::is_whitespace)
            {
                break;
            }
            start = previous;
        }
        if start != end {
            self.yank = self.buffer[start..end].to_string();
            self.buffer.replace_range(start..end, "");
            self.cursor = start;
            self.reset_selection();
        }
    }

    pub(super) fn kill_to_end(&mut self) {
        if self.cursor < self.buffer.len() {
            self.yank = self.buffer[self.cursor..].to_string();
            self.buffer.truncate(self.cursor);
            self.reset_selection();
        }
    }

    pub(super) fn kill_to_start(&mut self) {
        if self.cursor > 0 {
            self.yank = self.buffer[..self.cursor].to_string();
            self.buffer.replace_range(..self.cursor, "");
            self.cursor = 0;
            self.reset_selection();
        }
    }

    pub(super) fn insert_yank(&mut self) {
        if !self.yank.is_empty() {
            self.buffer.insert_str(self.cursor, &self.yank);
            self.cursor += self.yank.len();
            self.reset_selection();
        }
    }

    pub(super) fn reset_selection(&mut self) {
        self.selected = 0;
    }

    pub(super) fn result(&self, reason: ExitReason) -> RunResult {
        RunResult {
            reason,
            buffer: self.buffer.clone(),
            cursor: self.buffer[..self.cursor].chars().count(),
        }
    }
}
