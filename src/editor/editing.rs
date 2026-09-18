use crate::input::Key;

use super::{EditorState, ExitReason, RunResult};

impl EditorState {
    pub(super) fn handle_key(&mut self, key: Key) -> Option<RunResult> {
        let reveal_overlay = !matches!(key, Key::Escape | Key::Clear);
        match key {
            Key::Character(character) => self.insert_character(character),
            Key::Enter => return Some(self.result(ExitReason::Accepted)),
            Key::Tab => {
                if !self.apply_selected() {
                    return Some(self.result(ExitReason::DelegateTab));
                }
            }
            Key::Backspace => self.backspace(),
            Key::Delete => self.delete(),
            Key::WordBackspace => self.delete_word_back(),
            Key::KillToEnd => self.kill_to_end(),
            Key::KillToStart => self.kill_to_start(),
            Key::Yank => self.insert_yank(),
            Key::Left => self.move_left(),
            Key::Right => self.move_right(),
            Key::WordLeft => self.move_word_left(),
            Key::WordRight => self.move_word_right(),
            Key::Home => self.cursor = 0,
            Key::End => self.cursor = self.buffer.len(),
            Key::Up => {
                if !self.move_selection(-1) {
                    return Some(self.result(ExitReason::DelegateUp));
                }
            }
            Key::Down => {
                if !self.move_selection(1) {
                    return Some(self.result(ExitReason::DelegateDown));
                }
            }
            Key::Clear => self.toggle_overlay(),
            Key::Cancel => return Some(self.result(ExitReason::Interrupted)),
            Key::Escape => self.hide_overlay(),
            Key::Eof => {
                if self.buffer.is_empty() {
                    return Some(self.result(ExitReason::DelegateEof));
                }
                self.delete();
            }
        }
        if reveal_overlay {
            self.show_overlay();
        }
        self.note_activity();
        self.request_completion();
        None
    }
}
