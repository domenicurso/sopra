use sopra_completion::{CompletionEngine, ranking};

use super::EditorState;

impl EditorState {
    pub(super) fn request_completion(&mut self) {
        self.syntax = if self.syntax_enabled {
            crate::syntax::highlight_at(&self.buffer, &self.cwd, self.cursor)
        } else {
            Vec::new()
        };
        if self.buffer.trim().is_empty() || self.completions.suspended() {
            self.clear_completion_state();
            return;
        }
        if self.completions.engine.is_none() {
            self.refresh_suggestions();
            return;
        }
        let cursor_chars = self.buffer[..self.cursor].chars().count();
        let (context_line, context_cursor) = ranking::broad_context(&self.buffer, cursor_chars);
        let key = format!("{context_line}\0{context_cursor}\0{}", self.cwd.display());
        if key != self.completions.key {
            self.completions.key = key;
            self.completions.source.clear();
            self.completions.items.clear();
            self.reset_selection();
            self.completions.elapsed = std::time::Duration::ZERO;
        }
        if let Some(completion) = self.completions.engine.as_mut() {
            completion.request(&self.buffer, cursor_chars, &self.cwd);
        }
        self.refresh_suggestions();
    }

    pub(super) fn poll_completion(&mut self) {
        let responses = self
            .completions
            .engine
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
        if self.completions.suspended() {
            return;
        }
        let cursor_chars = self.buffer[..self.cursor].chars().count();
        if response.line != self.buffer
            || response.cursor != cursor_chars
            || response.generation < self.completions.generation
        {
            return;
        }
        self.completions.generation = response.generation;
        self.completions.source = response.items;
        if response.generation > self.completions.latency_generation {
            self.completions.latency_generation = response.generation;
            self.completions.elapsed = response.elapsed;
        } else if response.generation == self.completions.latency_generation {
            self.completions.elapsed = self.completions.elapsed.max(response.elapsed);
        }
        self.refresh_suggestions();
    }

    pub(super) fn refresh_suggestions(&mut self) {
        if self.completions.suspended() {
            self.completions.items.clear();
            self.completions.focus.select(None);
            self.completions.scroll = 0;
            return;
        }
        let previous = self
            .completions
            .focus
            .selected()
            .and_then(|index| self.completions.items.get(index))
            .map(|item| (item.kind, item.insert.clone()));
        self.completions.items = self.history.completions(&self.buffer, self.cursor);
        self.completions.items.extend(ranking::rank_for_buffer(
            &self.completions.source,
            &self.buffer,
            self.cursor,
        ));
        self.restore_completion_focus(previous);
    }

    fn restore_completion_focus(
        &mut self,
        previous: Option<(sopra_completion::CompletionKind, String)>,
    ) {
        if self.completions.items.is_empty() {
            self.completions.focus.select(None);
            self.completions.scroll = 0;
        } else if self.completions.focus.armed() {
            self.completions.focus.select(Some(
                previous
                    .filter(|(kind, _)| *kind != sopra_completion::CompletionKind::History)
                    .and_then(|(kind, insert)| {
                        self.completions
                            .items
                            .iter()
                            .position(|item| item.kind == kind && item.insert == insert)
                    })
                    .or_else(|| self.history_choice_index())
                    .or(self.completions.focus.selected())
                    .unwrap_or_default()
                    .min(self.completions.items.len().saturating_sub(1)),
            ));
            self.completions.scroll = self
                .completions
                .scroll
                .min(self.completions.items.len().saturating_sub(1));
        } else {
            self.completions.focus.select(None);
            self.completions.scroll = 0;
        }
    }

    fn clear_completion_state(&mut self) {
        self.completions.source.clear();
        self.completions.items.clear();
        self.completions.focus.select(None);
        self.completions.scroll = 0;
        self.completions.elapsed = std::time::Duration::ZERO;
    }

    pub(super) fn resume_completion(&mut self) {
        if self.completions.engine.is_none() || !self.completions.suspended() {
            return;
        }
        self.completions.set_suspended(false);
        self.completions.visible = true;
        self.completions.key.clear();
        self.clear_completion_state();
    }

    #[cfg(test)]
    pub(crate) fn set_completion_source_for_test(
        &mut self,
        items: Vec<sopra_completion::CompletionItem>,
    ) {
        self.completions.source = items;
        self.refresh_suggestions();
    }
}
