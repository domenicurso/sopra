use sopra_completion::{CompletionItem, CompletionKind, CompletionSource};

use super::{super::EditorState, snapshot};
use crate::actions::Action;

pub(super) struct Search {
    draft: String,
    cursor: usize,
    suspended: bool,
    visible: bool,
}

impl EditorState {
    pub(crate) fn searching_history(&self) -> bool {
        self.history.search.is_some()
    }

    pub(in crate::editor) fn history_search_input(&mut self, key: Action) -> bool {
        if !self.searching_history() {
            if key != Action::SearchHistory || self.widgets.active() {
                return false;
            }
            self.begin_history_search();
            return true;
        }
        let previous = self.buffer.clone();
        match key {
            Action::Escape | Action::Cancel => self.finish_history_search(false),
            Action::Enter | Action::Tab => self.finish_history_search(true),
            Action::Up | Action::PreviousSuggestion => {
                self.move_selection(-1);
            }
            Action::Down | Action::NextSuggestion | Action::BackTab | Action::SearchHistory => {
                self.move_selection(1);
            }
            Action::Character(ch) => {
                self.buffer.insert(self.cursor, ch);
                self.cursor += ch.len_utf8();
            }
            Action::ToggleMouseEditing | Action::PromptWidgets | Action::Clear => {}
            _ => {
                self.dispatch_search_edit(key);
            }
        }
        if self.searching_history() && self.buffer != previous {
            self.refresh_history_search();
        }
        self.note_activity();
        true
    }

    fn dispatch_search_edit(&mut self, key: Action) {
        match key {
            Action::Left => self.move_left(),
            Action::Right => self.move_right(),
            Action::WordLeft => self.move_word_left(),
            Action::WordRight => self.move_word_right(),
            Action::Home => self.cursor = 0,
            Action::End => self.cursor = self.buffer.len(),
            Action::Backspace => {
                if let Some(start) =
                    super::super::navigation::previous_grapheme_start(&self.buffer, self.cursor)
                {
                    self.buffer.replace_range(start..self.cursor, "");
                    self.cursor = start;
                    self.reset_selection();
                }
            }
            Action::Delete | Action::Eof => self.delete(),
            Action::WordBackspace => self.delete_word_back(),
            Action::KillToEnd => self.kill_to_end(),
            Action::KillToStart => self.kill_to_start(),
            Action::Yank => self.insert_yank(),
            _ => {}
        }
    }

    fn begin_history_search(&mut self) {
        self.history.search = Some(Search {
            draft: self.buffer.clone(),
            cursor: self.cursor,
            suspended: self.completions.suspended(),
            visible: self.completions.visible,
        });
        self.cursor = self.buffer.len();
        self.text_selection.clear();
        self.syntax.clear();
        self.completions.visible = true;
        self.refresh_history_search();
    }

    pub(in crate::editor) fn refresh_history_search(&mut self) {
        let query = self.buffer.to_lowercase();
        self.completions.items = self
            .history
            .entries
            .iter()
            .filter(|entry| entry.command.to_lowercase().contains(&query))
            .map(|entry| {
                CompletionItem::with_range(
                    entry.command.replace('\n', " \\\\ "),
                    entry.command.clone(),
                    Some(snapshot::description(entry)),
                    CompletionKind::History,
                    0..self.buffer.len(),
                    CompletionSource::ShellContext,
                )
            })
            .collect();
        self.completions.focus.set_armed(true);
        self.completions
            .focus
            .select((!self.completions.items.is_empty()).then_some(0));
        self.completions.scroll = 0;
    }

    fn finish_history_search(&mut self, accept: bool) {
        let selected = self
            .selected_index()
            .and_then(|i| self.completions.items.get(i))
            .map(|item| item.insert.clone());
        if accept && selected.is_none() {
            return;
        }
        let Some(search) = self.history.search.take() else {
            return;
        };
        self.buffer = if accept {
            selected.unwrap_or(search.draft)
        } else {
            search.draft
        };
        self.cursor = if accept {
            self.buffer.len()
        } else {
            search.cursor
        };
        if accept {
            self.history.recall = None;
        }
        self.completions.set_suspended(search.suspended);
        self.completions.visible = search.visible;
        self.completions.source.clear();
        self.completions.items.clear();
        self.completions.key.clear();
        self.disarm_completion();
        self.request_completion();
    }
}
