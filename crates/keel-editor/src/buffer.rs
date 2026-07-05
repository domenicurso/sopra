use keel_core::{EditorSnapshot, Key};
use unicode_segmentation::UnicodeSegmentation;

use crate::EditResult;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EditorBuffer {
    buffer: String,
    cursor: usize,
}

impl EditorBuffer {
    pub fn from_string(initial: impl Into<String>) -> Self {
        let buffer = normalize_pasted_text(initial.into());
        let cursor = grapheme_count(&buffer);
        Self { buffer, cursor }
    }

    pub fn from_parts(initial: impl Into<String>, cursor: usize) -> Self {
        let buffer = normalize_pasted_text(initial.into());
        let cursor = cursor.min(grapheme_count(&buffer));
        Self { buffer, cursor }
    }

    pub fn snapshot(&self) -> EditorSnapshot {
        EditorSnapshot {
            buffer: self.buffer.clone(),
            cursor: self.cursor,
            selection: None,
        }
    }

    pub fn apply_key(&mut self, key: Key) -> EditResult {
        match key {
            Key::Char(ch) => {
                self.insert_str(&ch.to_string());
                EditResult::Continue
            }
            Key::Backspace => {
                self.backspace();
                EditResult::Continue
            }
            Key::Delete => {
                self.delete();
                EditResult::Continue
            }
            Key::Left => {
                self.move_left();
                EditResult::Continue
            }
            Key::Right => {
                self.move_right();
                EditResult::Continue
            }
            Key::Up | Key::Down => EditResult::Continue,
            Key::Home => {
                self.move_home();
                EditResult::Continue
            }
            Key::End => {
                self.move_end();
                EditResult::Continue
            }
            Key::Tab => {
                self.insert_str("\t");
                EditResult::Continue
            }
            Key::Enter => EditResult::Submit(self.buffer.clone()),
            Key::CtrlC => {
                self.clear();
                EditResult::Continue
            }
            Key::Esc => EditResult::Cancel,
        }
    }

    pub fn insert_str(&mut self, text: &str) {
        let normalized = normalize_pasted_text(text.to_string());
        if normalized.is_empty() {
            return;
        }

        let byte_index = byte_index_for_grapheme(&self.buffer, self.cursor);
        self.buffer.insert_str(byte_index, &normalized);
        self.cursor += grapheme_count(&normalized);
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }

        let current_byte = byte_index_for_grapheme(&self.buffer, self.cursor);
        let previous_byte = byte_index_for_grapheme(&self.buffer, self.cursor - 1);
        self.buffer.replace_range(previous_byte..current_byte, "");
        self.cursor -= 1;
    }

    fn delete(&mut self) {
        let graphemes = grapheme_count(&self.buffer);
        if self.cursor >= graphemes {
            return;
        }

        let start = byte_index_for_grapheme(&self.buffer, self.cursor);
        let end = byte_index_for_grapheme(&self.buffer, self.cursor + 1);
        self.buffer.replace_range(start..end, "");
    }

    fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
    }

    fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    fn move_right(&mut self) {
        self.cursor = (self.cursor + 1).min(grapheme_count(&self.buffer));
    }

    fn move_home(&mut self) {
        self.cursor = 0;
    }

    fn move_end(&mut self) {
        self.cursor = grapheme_count(&self.buffer);
    }
}

fn byte_index_for_grapheme(text: &str, grapheme_index: usize) -> usize {
    if grapheme_index == 0 {
        return 0;
    }

    UnicodeSegmentation::grapheme_indices(text, true)
        .nth(grapheme_index)
        .map(|(byte_index, _)| byte_index)
        .unwrap_or(text.len())
}

fn grapheme_count(text: &str) -> usize {
    UnicodeSegmentation::graphemes(text, true).count()
}

fn normalize_pasted_text(mut input: String) -> String {
    for marker in ["\u{1b}[200~", "\u{1b}[201~"] {
        input = input.replace(marker, "");
    }

    if input.contains("[200~") && input.contains("[201~") {
        input = input.replace("[200~", "");
        input = input.replace("[201~", "");
    }

    input
}

#[cfg(test)]
mod tests {
    use super::EditorBuffer;
    use crate::EditResult;
    use keel_core::Key;

    #[test]
    fn edits_and_submits_a_line() {
        let mut buffer = EditorBuffer::default();
        assert_eq!(buffer.apply_key(Key::Char('e')), EditResult::Continue);
        assert_eq!(buffer.apply_key(Key::Char('c')), EditResult::Continue);
        assert_eq!(buffer.apply_key(Key::Char('h')), EditResult::Continue);
        assert_eq!(buffer.apply_key(Key::Char('o')), EditResult::Continue);
        assert_eq!(buffer.apply_key(Key::Char(' ')), EditResult::Continue);
        assert_eq!(buffer.apply_key(Key::Char('x')), EditResult::Continue);
        assert_eq!(buffer.apply_key(Key::Left), EditResult::Continue);
        assert_eq!(buffer.apply_key(Key::Char('y')), EditResult::Continue);
        assert_eq!(
            buffer.apply_key(Key::Enter),
            EditResult::Submit("echo yx".to_string())
        );
    }

    #[test]
    fn paste_respects_cursor_position() {
        let mut buffer = EditorBuffer::from_string("ls");
        assert_eq!(buffer.apply_key(Key::Home), EditResult::Continue);
        buffer.insert_str("sudo ");

        let snapshot = buffer.snapshot();
        assert_eq!(snapshot.buffer, "sudo ls");
        assert_eq!(snapshot.cursor, 5);
    }

    #[test]
    fn ctrl_c_clears_editor_session() {
        let mut buffer = EditorBuffer::from_string("echo hi");

        assert_eq!(buffer.apply_key(Key::CtrlC), EditResult::Continue);
        assert_eq!(buffer.snapshot().buffer, "");
    }

    #[test]
    fn strips_literal_bracketed_paste_markers() {
        let buffer = EditorBuffer::from_string("[200~pwd[201~");

        let snapshot = buffer.snapshot();
        assert_eq!(snapshot.buffer, "pwd");
        assert_eq!(snapshot.cursor, 3);
    }

    #[test]
    fn backspace_removes_full_grapheme_cluster() {
        let mut buffer = EditorBuffer::from_string("👍🏽a");
        assert_eq!(buffer.apply_key(Key::Backspace), EditResult::Continue);
        assert_eq!(buffer.snapshot().buffer, "👍🏽");
        assert_eq!(buffer.apply_key(Key::Backspace), EditResult::Continue);
        assert_eq!(buffer.snapshot().buffer, "");
    }
}
