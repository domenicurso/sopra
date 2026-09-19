use std::time::{Duration, Instant};

use super::{
    CompletionEngine, CompletionItem, CompletionResponse, CompletionResponseSource, Request,
    engine, parser,
};

impl CompletionEngine {
    pub(super) fn send_local(&mut self, request: &Request) -> bool {
        let started = Instant::now();
        let result = self.local.complete(request);
        let engine::LocalResult {
            items,
            prefetch,
            help,
        } = result;
        let _ = self
            .response_tx
            .send(self.local_response(request, items, started.elapsed()));
        if let Some(help) = help {
            self.enqueue_help(help);
        }
        prefetch
    }

    pub(super) fn schedule_help(&mut self, request: &Request) {
        let Some(invocation) = parser::invocation(request) else {
            return;
        };
        if !self.local.needs_graph(&invocation.key) {
            return;
        }
        self.enqueue_help(parser::request_for(request, &invocation));
    }

    pub(super) fn enqueue_help(&mut self, help: parser::HelpRequest) {
        let key = help.id();
        self.help_latest.insert(key.clone(), help.request.clone());
        if self.help_pending.insert(key.clone()) && self.help_requests.send(help).is_err() {
            self.help_pending.remove(&key);
            self.help_latest.remove(&key);
        }
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
