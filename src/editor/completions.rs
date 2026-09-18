use crate::completion::{
    CompletionEngine, CompletionItem, CompletionKind, CompletionResponseSource, ranking,
};

use super::EditorState;

impl EditorState {
    pub(super) fn request_completion(&mut self) {
        self.syntax = crate::syntax::highlight(&self.buffer, &self.cwd);
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
            self.zshrs_source = merge_response_metadata(&self.zshrs_source, items);
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
        self.completion_source = merge_items(&self.completion_source, &self.zshrs_source);
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

    #[cfg(test)]
    pub(crate) fn set_completion_source_for_test(&mut self, items: Vec<CompletionItem>) {
        self.completion_source = items;
        self.refresh_suggestions();
    }
}

fn merge_items(local: &[CompletionItem], zshrs: &[CompletionItem]) -> Vec<CompletionItem> {
    let local = local
        .iter()
        .filter(|item| item.source != crate::completion::CompletionSource::Zshrs)
        .cloned()
        .collect::<Vec<_>>();
    let rust_filesystem = local.iter().any(|item| {
        item.source == crate::completion::CompletionSource::Filesystem
            && matches!(item.kind, CompletionKind::File | CompletionKind::Directory)
    });
    let mut merged = local;
    for item in zshrs {
        if rust_filesystem && matches!(item.kind, CompletionKind::File | CompletionKind::Directory)
        {
            continue;
        }
        let duplicate = merged.iter_mut().find(|existing| same_item(existing, item));
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

fn merge_response_metadata(
    previous: &[CompletionItem],
    response: Vec<CompletionItem>,
) -> Vec<CompletionItem> {
    response
        .into_iter()
        .map(|mut item| {
            if let Some(previous) = previous
                .iter()
                .find(|candidate| same_item(candidate, &item))
            {
                if item.description.is_none() {
                    item.description = previous.description.clone();
                }
                if item.location.is_none() {
                    item.location = previous.location.clone();
                }
                if item.group.is_none() {
                    item.group = previous.group.clone();
                }
            }
            item
        })
        .collect()
}

fn same_item(left: &CompletionItem, right: &CompletionItem) -> bool {
    left.display == right.display && left.insert == right.insert && left.kind == right.kind
}
#[cfg(test)]
mod tests {
    use super::{merge_items, merge_response_metadata};
    use crate::completion::{CompletionItem, CompletionKind};

    #[test]
    fn zshrs_metadata_replaces_a_local_placeholder() {
        let local = [CompletionItem::new(
            "git",
            "path",
            "git",
            CompletionKind::Generic,
        )];
        let zshrs = [CompletionItem::new(
            "git",
            "/usr/bin/git",
            "git",
            CompletionKind::Generic,
        )];
        let merged = merge_items(&local, &zshrs);
        assert_eq!(merged[0].description.as_deref(), Some("/usr/bin/git"));
    }

    #[test]
    fn partial_zshrs_updates_keep_metadata_from_an_earlier_response() {
        let previous = vec![CompletionItem::new(
            "git",
            "/usr/bin/git",
            "git",
            CompletionKind::Generic,
        )];
        let response = vec![CompletionItem::new(
            "git",
            "",
            "git",
            CompletionKind::Generic,
        )];
        let merged = merge_response_metadata(&previous, response);
        assert_eq!(merged[0].description.as_deref(), Some("/usr/bin/git"));
    }
}
