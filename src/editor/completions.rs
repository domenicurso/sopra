use crate::completion::{CompletionEngine, CompletionItem, CompletionResponseSource, ranking};

use super::EditorState;

impl EditorState {
    pub(super) fn request_completion(&mut self) {
        self.syntax = crate::syntax::highlight(&self.buffer, &self.cwd);
        if self.completion.is_none() {
            return;
        }
        let cursor_chars = self.buffer[..self.cursor].chars().count();
        let (provider_line, provider_cursor) = ranking::broad_context(&self.buffer, cursor_chars);
        let key = format!("{provider_line}\0{provider_cursor}\0{}", self.cwd.display());
        if key != self.completion_key {
            self.completion_key = key;
            self.completion_source.clear();
            self.zshrs_source.clear();
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
            self.zshrs_source = response.items;
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
        self.completion_source = merge_items(&self.completion_source, &self.zshrs_source);
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

fn merge_items(local: &[CompletionItem], zshrs: &[CompletionItem]) -> Vec<CompletionItem> {
    let mut merged = local.to_vec();
    for item in zshrs {
        let duplicate = merged.iter_mut().find(|existing| {
            existing.display == item.display
                && existing.insert == item.insert
                && existing.kind == item.kind
        });
        if let Some(existing) = duplicate {
            if item.description.is_some() || item.location.is_some() {
                *existing = item.clone();
            }
        } else {
            merged.push(item.clone());
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::merge_items;
    use crate::completion::{CompletionItem, CompletionKind};

    #[test]
    fn zshrs_metadata_replaces_a_local_placeholder() {
        let local = [CompletionItem::new(
            "git",
            "path",
            "git",
            CompletionKind::Generic,
        )];
        let provider = [CompletionItem::new(
            "git",
            "/usr/bin/git",
            "git",
            CompletionKind::Generic,
        )];
        let merged = merge_items(&local, &provider);
        assert_eq!(merged[0].description.as_deref(), Some("/usr/bin/git"));
    }
}
