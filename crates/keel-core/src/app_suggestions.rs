use neo_frizbee::{Config, match_list_indices};
use unicode_width::UnicodeWidthStr;

use crate::{AppState, Suggestion};

pub const SUGGESTION_VIEWPORT_ROWS: usize = 12;

impl AppState {
    pub fn selected(&self) -> Option<&Suggestion> {
        self.selected_suggestion
            .and_then(|selected| self.suggestions.get(selected))
    }

    pub fn suggestions_visible(&self) -> bool {
        !self.buffer.text().is_empty() && !self.overlay_dismissed && !self.suggestions.is_empty()
    }

    pub fn selected_replacement(&self) -> Option<&str> {
        self.selected()
            .or_else(|| (self.suggestions.len() == 1).then(|| &self.suggestions[0]))
            .map(Suggestion::replacement)
    }

    pub fn completion_source_available(&self) -> bool {
        self.completion_source_key.is_some()
    }

    pub fn suggestion_viewport_start(&self) -> usize {
        self.suggestion_scroll
    }

    pub fn completion_query(&self) -> String {
        let cursor = self.buffer.cursor_byte_offset();
        self.buffer.text()[..cursor]
            .rsplit(char::is_whitespace)
            .next()
            .unwrap_or_default()
            .to_string()
    }

    pub fn completion_context_key(&self) -> String {
        let text = self.buffer.text();
        let cursor = self.buffer.cursor_byte_offset();
        let prefix = &text[..cursor];
        let token_start = prefix
            .char_indices()
            .rev()
            .find(|(_, character)| character.is_whitespace())
            .map_or(0, |(offset, character)| offset + character.len_utf8());
        let token_prefix = &prefix[token_start..];
        let suffix = &text[cursor..];
        let token_end = suffix.find(char::is_whitespace).unwrap_or(suffix.len());
        let line_suffix = &suffix[token_end..];
        let option_prefix = if token_prefix.starts_with('-') {
            "-"
        } else {
            ""
        };
        format!("{}{}{}", &prefix[..token_start], option_prefix, line_suffix)
    }

    pub fn completion_token_width(&self) -> u16 {
        self.completion_query().width().min(u16::MAX as usize) as u16
    }

    pub fn set_suggestions(&mut self, suggestions: Vec<Suggestion>) -> bool {
        self.completion_source = suggestions;
        self.completion_source_key = Some(self.current_completion_context_key.clone());
        self.ranked_query = None;
        self.refresh_suggestions()
    }

    pub fn refresh_suggestions(&mut self) -> bool {
        if self.completion_source_key.as_deref()
            != Some(self.current_completion_context_key.as_str())
        {
            let changed = !self.suggestions.is_empty();
            self.suggestions.clear();
            self.ranked_query = None;
            self.selected_suggestion = None;
            self.suggestion_scroll = 0;
            self.dirty |= changed;
            return changed;
        }
        let query = self.completion_query();
        if self.ranked_query.as_deref() == Some(query.as_str()) {
            return false;
        }
        let suggestions = rank_suggestions(&self.completion_source, &query);
        self.ranked_query = Some(query);
        let had_selection = self.selected_suggestion.is_some();
        let changed = self.suggestions != suggestions;
        self.suggestions = suggestions;
        if changed {
            self.selected_suggestion = had_selection
                .then_some(0)
                .filter(|_| !self.suggestions.is_empty());
            self.suggestion_scroll = 0;
        } else if self
            .selected_suggestion
            .is_some_and(|selected| selected >= self.suggestions.len())
        {
            self.selected_suggestion = None;
            self.suggestion_scroll = 0;
        }
        self.dirty |= changed;
        changed
    }

    pub fn move_selection(&mut self, delta: isize) -> bool {
        if !self.suggestions_visible() {
            return false;
        }

        let len = self.suggestions.len() as isize;
        let next = match self.selected_suggestion {
            Some(selected) => (selected as isize + delta).rem_euclid(len) as usize,
            None if delta < 0 => len.saturating_sub(1) as usize,
            None => 0,
        };
        self.selected_suggestion = Some(next);
        self.update_suggestion_scroll(next);
        self.dirty = true;
        true
    }

    fn update_suggestion_scroll(&mut self, selected: usize) {
        let length = self.suggestions.len();
        let max_start = length.saturating_sub(SUGGESTION_VIEWPORT_ROWS);
        if max_start == 0 {
            self.suggestion_scroll = 0;
            return;
        }

        // Keep one row available in the direction of travel. The selected item
        // moves the viewport when it reaches the last visible row, then moves
        // it back when it reaches the first visible row.
        if selected >= self.suggestion_scroll + SUGGESTION_VIEWPORT_ROWS - 1 {
            let desired_start = selected.saturating_sub(SUGGESTION_VIEWPORT_ROWS - 2);
            self.suggestion_scroll = self.suggestion_scroll.max(desired_start).min(max_start);
        } else if selected <= self.suggestion_scroll {
            self.suggestion_scroll = self.suggestion_scroll.min(selected.saturating_sub(1));
        }
    }

    pub fn dismiss_overlay(&mut self) -> bool {
        let changed = self.suggestions_visible();
        self.overlay_dismissed = true;
        self.suppressed_overlay_text = Some(self.buffer.text().to_string());
        self.dirty |= changed;
        changed
    }

    pub fn suppress_overlay_for_text(&mut self, text: impl Into<String>) {
        self.overlay_dismissed = true;
        self.suppressed_overlay_text = Some(text.into());
        self.dirty = true;
    }

    pub fn clear(&mut self) -> bool {
        let changed = !self.buffer.text().is_empty() || self.buffer.cursor() != 0;
        self.buffer = crate::EditorBuffer::new("", 0);
        self.suggestions.clear();
        self.completion_source.clear();
        self.completion_source_key = None;
        self.current_completion_context_key = self.completion_context_key();
        self.ranked_query = None;
        self.selected_suggestion = None;
        self.suggestion_scroll = 0;
        self.overlay_dismissed = false;
        self.suppressed_overlay_text = None;
        self.dirty = true;
        changed
    }
}

fn rank_suggestions(suggestions: &[Suggestion], query: &str) -> Vec<Suggestion> {
    if query.is_empty() || suggestions.is_empty() {
        return suggestions.to_vec();
    }

    let labels = suggestions
        .iter()
        .map(|suggestion| suggestion.label.as_str())
        .collect::<Vec<_>>();
    let matches = match_list_indices(query, &labels, &Config::default());
    let mut ranked = Vec::with_capacity(matches.len());

    for matched in matches {
        let index = matched.index as usize;
        let Some(mut suggestion) = suggestions.get(index).cloned() else {
            continue;
        };
        suggestion.match_indices = matched.indices;
        ranked.push(suggestion);
    }

    ranked
}
