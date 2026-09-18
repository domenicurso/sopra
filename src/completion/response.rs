use std::time::{Duration, Instant};

use super::{
    CompletionEngine, CompletionItem, CompletionResponse, CompletionResponseSource, Request, help,
    local,
};

pub(super) struct LocalDecision {
    pub(super) handled: bool,
    pub(super) prefetch: bool,
}

impl CompletionEngine {
    pub(super) fn send_local(&mut self, request: &Request) -> LocalDecision {
        let started = Instant::now();
        let result = self.local.complete(request);
        let local::LocalResult {
            items,
            handled,
            prefetch,
            help,
        } = result;
        let _ = self
            .response_tx
            .send(self.local_response(request, items, started.elapsed()));
        if let Some(help) = help {
            let key = help.key.clone();
            self.help_latest.insert(key.clone(), help.request.clone());
            if self.help_pending.insert(key.clone()) && self.help_requests.send(help).is_err() {
                self.help_pending.remove(&key);
                self.help_latest.remove(&key);
            }
        }
        LocalDecision { handled, prefetch }
    }

    pub(super) fn schedule_help(&mut self, request: &Request) {
        let Some(invocation) = help::invocation(request) else {
            return;
        };
        if !self.local.needs_help(&invocation.key) {
            return;
        }
        let key = invocation.key.clone();
        if self.help_pending.insert(key.clone()) {
            let help = help::request_for(request, &invocation);
            self.help_latest.insert(key.clone(), help.request.clone());
            if self.help_requests.send(help).is_err() {
                self.help_pending.remove(&key);
                self.help_latest.remove(&key);
            }
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

    pub(super) fn send_zshrs(
        &self,
        request: &Request,
        items: Vec<CompletionItem>,
        elapsed: Duration,
    ) {
        let _ = self.response_tx.send(CompletionResponse {
            line: request.line.clone(),
            cursor: request.cursor,
            context_line: request.context_line.clone(),
            context_cursor: request.context_cursor,
            context_key: request.context_key.clone(),
            items,
            generation: request.generation,
            source: CompletionResponseSource::Zshrs,
            incomplete: false,
            elapsed,
        });
    }
}
