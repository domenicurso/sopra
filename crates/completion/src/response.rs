use std::time::{Duration, Instant};

use super::{
    CompletionEngine, CompletionItem, CompletionResponse, CompletionResponseSource, Request,
    engine, parser,
};

impl CompletionEngine {
    pub(super) fn send_local(&mut self, request: &Request) -> bool {
        self.filesystem_request = Some(request.clone());
        let started = Instant::now();
        let result = self.local.complete(request);
        let engine::LocalResult {
            items,
            prefetch,
            help,
            provider,
        } = result;
        if let Some(query) = provider {
            self.providers.submit(query);
        } else {
            self.providers.cancel();
        }
        let _ = self
            .response_tx
            .send(self.local_response(request, items, started.elapsed()));
        if let Some(help) = help {
            self.enqueue_help(help);
        }
        prefetch
    }

    pub(super) fn enqueue_help(&mut self, help: parser::HelpRequest) {
        let key = help.id();
        self.prefetch_keys.remove(&key);
        self.help_prefetch.retain(|queued| queued.id() != key);
        self.send_help(help, false);
    }

    pub(super) fn queue_prefetch(&mut self, help: parser::HelpRequest) {
        let key = help.id();
        if self.help_pending.contains(&key) || !self.prefetch_keys.insert(key) {
            return;
        }
        self.help_prefetch.push_back(help);
    }

    pub(super) fn pump_prefetch(&mut self) {
        if self.prefetch_inflight.is_some() {
            return;
        }
        while let Some(help) = self.help_prefetch.pop_front() {
            let key = help.id();
            self.prefetch_keys.remove(&key);
            if self.help_pending.contains(&key) {
                continue;
            }
            self.prefetch_inflight = Some(key);
            if !self.send_help(help, true) {
                self.prefetch_inflight = None;
                continue;
            }
            break;
        }
    }

    fn send_help(&mut self, help: parser::HelpRequest, background: bool) -> bool {
        let key = help.id();
        self.help_latest.insert(key.clone(), help.request.clone());
        if !self.help_pending.insert(key.clone()) {
            if !background {
                self.help_requests.promote(&key);
            }
            return true;
        }
        if !self.help_requests.send(help, background) {
            self.help_pending.remove(&key);
            self.help_latest.remove(&key);
            return false;
        }
        true
    }

    pub(super) fn local_response(
        &self,
        request: &Request,
        items: Vec<CompletionItem>,
        elapsed: Duration,
    ) -> CompletionResponse {
        CompletionResponse {
            line: request.line.clone(),
            cursor: request.cursor,
            context_line: request.context_line.clone(),
            context_cursor: request.context_cursor,
            context_key: request.context_key.clone(),
            items,
            generation: request.generation,
            source: CompletionResponseSource::Local,
            incomplete: false,
            elapsed,
        }
    }
}
