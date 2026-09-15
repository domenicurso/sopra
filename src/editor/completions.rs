#[cfg(test)]
use crate::completion::CompletionItem;
use crate::completion::{CompletionClient, ranking};

use super::EditorState;

impl EditorState {
    pub(super) fn request_completion(&mut self) {
        if self.completion.is_none() {
            return;
        }
        let cursor_chars = self.buffer[..self.cursor].chars().count();
        let (provider_line, provider_cursor) = ranking::broad_context(&self.buffer, cursor_chars);
        let key = format!("{provider_line}\0{provider_cursor}\0{}", self.cwd.display());
        if key != self.completion_key {
            self.completion_key = key;
            self.completion_source.clear();
            self.provider_source.clear();
            self.suggestions.clear();
            self.selected = 0;
        }
        if let Some(completion) = self.completion.as_mut() {
            completion.request(&self.buffer, cursor_chars, &self.cwd);
        }
    }

    pub(super) fn poll_completion(&mut self) {
        let responses = self
            .completion
            .as_ref()
            .map(CompletionClient::poll)
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        for response in responses {
            self.apply_completion(response);
        }
    }

    fn apply_completion(&mut self, response: crate::completion::CompletionResponse) {
        let cursor_chars = self.buffer[..self.cursor].chars().count();
        let (context_line, context_cursor) = ranking::broad_context(&self.buffer, cursor_chars);
        if response.provider {
            if response.context_line != context_line
                || response.context_cursor != context_cursor
                || response.generation < self.latest_provider_generation
            {
                return;
            }
            self.latest_provider_generation = response.generation;
            self.provider_source = response.items;
        } else {
            if response.line != self.buffer
                || response.cursor != cursor_chars
                || response.generation < self.latest_local_generation
            {
                return;
            }
            self.latest_local_generation = response.generation;
            self.completion_source = response.items;
        }
        self.merge_completion_sources();
    }

    fn merge_completion_sources(&mut self) {
        let mut merged = self.completion_source.clone();
        for item in &self.provider_source {
            let duplicate = merged.iter().any(|existing| {
                existing.label == item.label
                    && existing.replacement == item.replacement
                    && existing.kind == item.kind
            });
            if !duplicate {
                merged.push(item.clone());
            }
        }
        self.completion_source = merged;
        self.refresh_suggestions();
    }

    pub(super) fn refresh_suggestions(&mut self) {
        let query = self.query().to_string();
        self.suggestions = ranking::rank(&self.completion_source, &query);
        if self.suggestions.is_empty() {
            self.selected = 0;
        } else {
            self.selected = self.selected.min(self.suggestions.len() - 1);
        }
    }

    #[cfg(test)]
    pub(crate) fn set_completion_source_for_test(&mut self, items: Vec<CompletionItem>) {
        self.completion_source = items;
        self.refresh_suggestions();
    }
}
