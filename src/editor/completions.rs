use sopra_completion::{CompletionEngine, ranking};

use super::EditorState;

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
            self.suggestions.clear();
            self.reset_selection();
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

    fn apply_completion(&mut self, response: sopra_completion::CompletionResponse) {
        let cursor_chars = self.buffer[..self.cursor].chars().count();
        if response.line != self.buffer
            || response.cursor != cursor_chars
            || response.generation < self.latest_local_generation
        {
            return;
        }
        self.latest_local_generation = response.generation;
        self.completion_source = response.items;
        if response.generation > self.completion_latency_generation {
            self.completion_latency_generation = response.generation;
            self.completion_elapsed = response.elapsed;
        } else if response.generation == self.completion_latency_generation {
            self.completion_elapsed = self.completion_elapsed.max(response.elapsed);
        }
        self.refresh_suggestions();
    }

    pub(super) fn refresh_suggestions(&mut self) {
        self.suggestions =
            ranking::rank_for_buffer(&self.completion_source, &self.buffer, self.cursor);
        if self.suggestions.is_empty() {
            self.selected = None;
            self.suggestion_scroll = 0;
        } else if self.completion_armed {
            self.selected = Some(
                self.selected
                    .unwrap_or_default()
                    .min(self.suggestions.len().saturating_sub(1)),
            );
            self.suggestion_scroll = self
                .suggestion_scroll
                .min(self.suggestions.len().saturating_sub(1));
        } else {
            self.selected = None;
            self.suggestion_scroll = 0;
        }
    }

    fn clear_completion_state(&mut self) {
        self.completion_source.clear();
        self.suggestions.clear();
        self.selected = None;
        self.suggestion_scroll = 0;
        self.completion_elapsed = std::time::Duration::ZERO;
    }

    #[cfg(test)]
    pub(crate) fn set_completion_source_for_test(
        &mut self,
        items: Vec<sopra_completion::CompletionItem>,
    ) {
        self.completion_source = items;
        self.refresh_suggestions();
    }
}
