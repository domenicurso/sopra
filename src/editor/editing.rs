use crate::actions::Action;

use super::{EditorState, ExitReason, RunResult};

impl EditorState {
    pub(super) fn handle_key(&mut self, key: Action) -> Option<RunResult> {
        if key == Action::Ignore {
            return None;
        }
        if self.history_search_input(key) {
            return None;
        }
        match self.widgets.handle(key) {
            sopra::Action::Consumed => return None,
            sopra::Action::ChangeDirectory(directory) => {
                let mut result = self.result(ExitReason::ChangeDirectory);
                result.directory = Some(directory);
                return Some(result);
            }
            sopra::Action::Ignored => {}
        }
        if self.prepare_text_selection(key) {
            self.note_activity();
            self.request_completion();
            return None;
        }
        self.finish_history_recall_for(key);
        self.resume_completion_for(key);
        let reveal_overlay = !matches!(
            key,
            Action::Escape
                | Action::Clear
                | Action::Up
                | Action::Down
                | Action::PreviousSuggestion
                | Action::NextSuggestion
        );
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

    fn resume_completion_for(&mut self, key: Action) {
        if matches!(
            key,
            Action::Character(_)
                | Action::Backspace
                | Action::Delete
                | Action::WordBackspace
                | Action::KillToEnd
                | Action::KillToStart
                | Action::Yank
                | Action::Left
                | Action::Right
                | Action::WordLeft
                | Action::WordRight
                | Action::Home
                | Action::End
        ) {
            self.resume_completion();
        }
    }

    fn complete_tab(&mut self) -> Option<RunResult> {
        if self.completions.focus.selected().is_none() {
            if !self.arm_first_completion() {
                return Some(self.result(ExitReason::DelegateTab));
            }
            if self.completions.items.len() == 1 && !self.apply_selected() {
                return Some(self.result(ExitReason::DelegateTab));
            }
        } else if !self.apply_selected() {
            return Some(self.result(ExitReason::DelegateTab));
        }
        None
    }

    fn dispatch_key(&mut self, key: Action) -> Option<RunResult> {
        match key {
            Action::Character(character) => self.insert_character(character),
            Action::Mouse(_) | Action::Ignore => {}
            Action::Enter => return Some(self.result(ExitReason::Accepted)),
            Action::Tab => return self.complete_tab(),
            Action::BackTab => {}
            Action::PreviousSuggestion => self.cycle_history(-1),
            Action::NextSuggestion => self.cycle_history(1),
            Action::Backspace => self.backspace(),
            Action::Delete => self.delete(),
            Action::WordBackspace => self.delete_word_back(),
            Action::KillToEnd => self.kill_to_end(),
            Action::KillToStart => self.kill_to_start(),
            Action::Yank => self.insert_yank(),
            Action::Left
            | Action::Right
            | Action::WordLeft
            | Action::WordRight
            | Action::Home
            | Action::End
            | Action::Up
            | Action::Down => return self.navigate(key),
            Action::Clear => self.toggle_overlay(),
            Action::PromptWidgets | Action::ToggleMouseEditing | Action::SearchHistory => {}
            Action::Cancel => return Some(self.result(ExitReason::Interrupted)),
            Action::Escape => {
                self.history.dismiss(&self.buffer);
                self.hide_overlay();
                self.disarm_completion();
            }
            Action::Eof => {
                if self.buffer.is_empty() {
                    return Some(self.result(ExitReason::DelegateEof));
                }
                self.delete();
            }
        }
        None
    }
}
