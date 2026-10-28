use super::super::EditorState;
use sopra_completion::CompletionKind;

pub(super) struct Recall {
    index: usize,
    draft: String,
}

impl EditorState {
    pub(in crate::editor) fn history_choice_index(&self) -> Option<usize> {
        self.suggestions.iter().position(|item| {
            item.kind == CompletionKind::History
                && Some(&item.insert) == self.history.choice.as_ref()
        })
    }

    pub(in crate::editor) fn navigate_closed_history(&mut self, key: crate::input::Key) -> bool {
        use crate::input::Key;
        if !matches!(key, Key::Up | Key::Down)
            || (self.overlay_visible() && !self.suggestions.is_empty())
        {
            return false;
        }
        let direction = if key == Key::Up { -1 } else { 1 };
        if self.history.recall.is_none() && self.history_suffix().is_some() {
            self.cycle_history(direction);
        } else {
            if self.overlay_visible() {
                return false;
            }
            if !self.move_vertical(direction) {
                self.recall_history(direction);
            }
        }
        true
    }

    pub(in crate::editor) fn recall_history(&mut self, direction: i8) {
        if self.history.entries.is_empty() {
            return;
        }
        let next = match self.history.recall.take() {
            None if direction > 0 => return,
            None => Recall {
                index: 0,
                draft: self.buffer.clone(),
            },
            Some(recall) if direction > 0 && recall.index == 0 => {
                self.buffer = recall.draft;
                self.cursor = self.buffer.len();
                return;
            }
            Some(mut recall) => {
                recall.index = if direction < 0 {
                    (recall.index + 1).min(self.history.entries.len() - 1)
                } else {
                    recall.index - 1
                };
                recall
            }
        };
        self.buffer = self.history.entries[next.index].command.clone();
        self.cursor = self.buffer.len();
        self.history.recall = Some(next);
        self.text_selection.clear();
        self.completion_suspended = true;
        self.hide_overlay();
    }

    pub(in crate::editor) fn finish_history_recall_for(&mut self, key: crate::input::Key) {
        if !matches!(key, crate::input::Key::Up | crate::input::Key::Down) {
            self.history.recall = None;
        }
    }

    pub(in crate::editor) fn history_preview_index(&self) -> Option<usize> {
        let suffix = self.history_suffix()?;
        self.suggestions.iter().position(|item| {
            item.kind == CompletionKind::History
                && item.insert.strip_prefix(&self.buffer) == Some(suffix)
        })
    }

    pub(in crate::editor) fn cycle_history(&mut self, direction: i8) {
        if self.cursor != self.buffer.len() || self.widgets.active() {
            return;
        }
        let matches = self.history.matching(&self.buffer).collect::<Vec<_>>();
        if matches.is_empty() {
            return;
        }
        let current = self.history.chosen(&self.buffer);
        let index = matches
            .iter()
            .position(|entry| Some(entry.command.as_str()) == current)
            .unwrap_or_default();
        let index = if direction < 0 {
            index.checked_sub(1).unwrap_or(matches.len() - 1)
        } else {
            (index + 1) % matches.len()
        };
        self.history.choice = Some(matches[index].command.clone());
        self.history.dismissed = None;
        self.selected = self.history_choice_index();
        self.completion_armed = self.selected.is_some();
    }

    pub(in crate::editor) fn remember_history_selection(&mut self) {
        if let Some(item) = self
            .selected
            .and_then(|index| self.suggestions.get(index))
            .filter(|item| item.kind == CompletionKind::History)
        {
            self.history.choice = Some(item.insert.clone());
        }
    }
}
