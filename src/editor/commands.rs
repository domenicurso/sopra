use super::{EditorState, ExitReason, RunResult, navigation::previous_grapheme_start};

impl EditorState {
    pub(super) fn insert_character(&mut self, character: char) {
        super::pairs::insert(&mut self.buffer, &mut self.cursor, character);
        self.reset_selection();
    }

    pub(super) fn toggle_overlay(&mut self) {
        self.overlay_visible = !self.overlay_visible;
    }

    pub(super) fn hide_overlay(&mut self) {
        self.overlay_visible = false;
    }

    pub(super) fn show_overlay(&mut self) {
        self.overlay_visible = true;
    }

    pub(super) fn apply_selected(&mut self) -> bool {
        let Some(selected) = self.selected else {
            return false;
        };
        let Some(item) = self.suggestions.get(selected).cloned() else {
            return false;
        };
        let replace = item.replace;
        let explicit_range = replace.start != 0 || replace.end != 0;
        let (start, end) =
            if explicit_range && replace.start <= replace.end && replace.end <= self.buffer.len() {
                if self.buffer.is_char_boundary(replace.start)
                    && self.buffer.is_char_boundary(replace.end)
                {
                    (replace.start, replace.end)
                } else {
                    self.current_token_range()
                }
            } else {
                self.current_token_range()
            };
        let inserted = format!("{}{}", item.insert, item.suffix);
        let cursor_offset = item
            .cursor_offset
            .unwrap_or(inserted.len())
            .min(inserted.len());
        // Assignment keys and directory paths are incomplete shell words, so keep them open.
        let add_space = cursor_offset == inserted.len()
            && end == self.buffer.len()
            && inserted.chars().next_back().is_some_and(|character| {
                !character.is_whitespace() && character != '=' && character != '/'
            });
        let replacement = if add_space {
            format!("{inserted} ")
        } else {
            inserted.clone()
        };
        self.buffer.replace_range(start..end, &replacement);
        self.cursor = start + cursor_offset + usize::from(add_space);
        self.reset_selection();
        true
    }

    pub(super) fn backspace(&mut self) {
        let Some(start) = previous_grapheme_start(&self.buffer, self.cursor) else {
            return;
        };
        let delete_pair = super::pairs::should_delete_pair(&self.buffer, start, self.cursor);
        let end = if delete_pair {
            self.buffer[self.cursor..]
                .chars()
                .next()
                .map_or(self.cursor, |character| self.cursor + character.len_utf8())
        } else {
            self.cursor
        };
        self.buffer.replace_range(start..end, "");
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
        self.selected = self.completion_armed.then_some(0);
        self.suggestion_scroll = 0;
    }

    pub(super) fn arm_first_completion(&mut self) -> bool {
        if self.suggestions.is_empty() || !self.overlay_visible {
            return false;
        }
        self.completion_armed = true;
        self.selected = Some(0);
        self.suggestion_scroll = 0;
        true
    }

    pub(super) fn disarm_completion(&mut self) {
        self.completion_armed = false;
        self.selected = None;
        self.suggestion_scroll = 0;
    }

    pub(super) fn result(&self, reason: ExitReason) -> RunResult {
        RunResult {
            reason,
            buffer: self.buffer.clone(),
            cursor: self.buffer[..self.cursor].chars().count(),
            highlights: crate::syntax::highlight_without_cursor(&self.buffer, &self.cwd),
        }
    }
}
