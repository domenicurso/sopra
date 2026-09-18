use crate::completion::{CompletionEngine, CompletionResponseSource, ranking};

use super::{EditorState, completion_merge};

impl EditorState {
    pub(super) fn request_completion(&mut self) {
        self.syntax = crate::syntax::highlight_at(&self.buffer, &self.cwd, self.cursor);
        if self.buffer.trim().is_empty() {
            self.clear_completion_state();
            return;
        }
        if self.completion.is_none() {
            return;
        }
        let cursor_chars = self.buffer[..self.cursor].chars().count();
        let (context_line, context_cursor) = ranking::broad_context(&self.buffer, cursor_chars);
        let key = format!("{context_line}\0{context_cursor}\0{}", self.cwd.display());
        if key != self.completion_key {
            self.completion_key = key;
            self.completion_source.clear();
            self.zshrs_source.clear();
            self.suggestions.clear();
            self.selected = 0;
            self.suggestion_scroll = 0;
            self.completion_elapsed = std::time::Duration::ZERO;
        }
        if let Some(completion) = self.completion.as_mut() {
            completion.request(&self.buffer, cursor_chars, &self.cwd);
        }
    }

    pub(super) fn poll_completion(&mut self) {
        let responses = self
            .completion
            .as_mut()
            .map(CompletionEngine::poll)
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
        if response.source == CompletionResponseSource::Zshrs {
            if response.context_line != context_line
                || response.context_cursor != context_cursor
                || response.generation < self.latest_zshrs_generation
            {
                return;
            }
            self.latest_zshrs_generation = response.generation;
            let (start, end) = ranking::token_range(&self.buffer, self.cursor);
            let replace = start..end;
            let items = crate::completion::rebind(&response.items, replace);
            self.zshrs_source = completion_merge::response_metadata(&self.zshrs_source, items);
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
        if response.generation > self.completion_latency_generation {
            self.completion_latency_generation = response.generation;
            self.completion_elapsed = response.elapsed;
        } else if response.generation == self.completion_latency_generation {
            self.completion_elapsed = self.completion_elapsed.max(response.elapsed);
        }
        self.merge_completion_sources();
    }

    fn merge_completion_sources(&mut self) {
        self.completion_source =
            completion_merge::items(&self.completion_source, &self.zshrs_source);
        self.refresh_suggestions();
    }

    pub(super) fn refresh_suggestions(&mut self) {
        let query = self.query().to_string();
        self.suggestions = ranking::rank(&self.completion_source, &query);
        if self.suggestions.is_empty() {
            self.selected = 0;
            self.suggestion_scroll = 0;
        } else {
            self.selected = self.selected.min(self.suggestions.len() - 1);
            self.suggestion_scroll = self
                .suggestion_scroll
                .min(self.suggestions.len().saturating_sub(1));
        }
    }

    fn clear_completion_state(&mut self) {
        self.completion_source.clear();
        self.zshrs_source.clear();
        self.suggestions.clear();
        self.selected = 0;
        self.suggestion_scroll = 0;
        self.completion_elapsed = std::time::Duration::ZERO;
    }

    #[cfg(test)]
    pub(crate) fn set_completion_source_for_test(
        &mut self,
        items: Vec<crate::completion::CompletionItem>,
    ) {
        self.completion_source = items;
        self.refresh_suggestions();
    }
}
