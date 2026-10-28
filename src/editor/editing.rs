use crate::input::Key;

use super::{EditorState, ExitReason, RunResult};

impl EditorState {
    pub(super) fn handle_key(&mut self, key: Key) -> Option<RunResult> {
        if key == Key::Ignore {
            return None;
        }
        match self.widgets.handle(key) {
            crate::prompt::widgets::WidgetAction::Consumed => return None,
            crate::prompt::widgets::WidgetAction::ChangeDirectory(directory) => {
                let mut result = self.result(ExitReason::ChangeDirectory);
                result.directory = Some(directory);
                return Some(result);
            }
            crate::prompt::widgets::WidgetAction::Ignored => {}
        }
        if self.prepare_text_selection(key) {
            self.note_activity();
            self.request_completion();
            return None;
        }
        self.resume_completion_for(key);
        let reveal_overlay = !matches!(key, Key::Escape | Key::Clear);
        if let Some(result) = self.dispatch_key(key) {
            return Some(result);
        }
        if reveal_overlay {
            self.show_overlay();
        }
        self.note_activity();
        self.request_completion();
        None
    }

    fn resume_completion_for(&mut self, key: Key) {
        if matches!(
            key,
            Key::Character(_)
                | Key::Backspace
                | Key::Delete
                | Key::WordBackspace
                | Key::KillToEnd
                | Key::KillToStart
                | Key::Yank
                | Key::Left
                | Key::Right
                | Key::WordLeft
                | Key::WordRight
                | Key::Home
                | Key::End
        ) {
            self.resume_completion();
        }
    }

    fn dispatch_key(&mut self, key: Key) -> Option<RunResult> {
        match key {
            Key::Character(character) => self.insert_character(character),
            Key::Mouse(_) | Key::Ignore => {}
            Key::Enter => return Some(self.result(ExitReason::Accepted)),
            Key::Tab => {
                if self.selected.is_none() {
                    if !self.arm_first_completion() {
                        return Some(self.result(ExitReason::DelegateTab));
                    }
                    if self.suggestions.len() == 1 && !self.apply_selected() {
                        return Some(self.result(ExitReason::DelegateTab));
                    }
                } else if !self.apply_selected() {
                    return Some(self.result(ExitReason::DelegateTab));
                }
            }
            Key::BackTab => {}
            Key::Backspace => self.backspace(),
            Key::Delete => self.delete(),
            Key::WordBackspace => self.delete_word_back(),
            Key::KillToEnd => self.kill_to_end(),
            Key::KillToStart => self.kill_to_start(),
            Key::Yank => self.insert_yank(),
            Key::Left
            | Key::Right
            | Key::WordLeft
            | Key::WordRight
            | Key::Home
            | Key::End
            | Key::Up
            | Key::Down => return self.navigate(key),
            Key::Clear => self.toggle_overlay(),
            Key::PromptWidgets => {}
            Key::Cancel => return Some(self.result(ExitReason::Interrupted)),
            Key::Escape => {
                self.hide_overlay();
                self.disarm_completion();
            }
            Key::Eof => {
                if self.buffer.is_empty() {
                    return Some(self.result(ExitReason::DelegateEof));
                }
                self.delete();
            }
        }
        None
    }
}
