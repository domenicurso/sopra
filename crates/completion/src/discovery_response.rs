use crate::{CompletionEngine, CompletionResponse, CompletionSource, parser};

impl CompletionEngine {
    pub fn poll(&mut self) -> impl Iterator<Item = CompletionResponse> {
        let mut responses = Vec::new();
        if self.local.poll_filesystem()
            && let Some(request) = self.filesystem_request.clone()
            && request.generation == self.generation
        {
            self.send_local(&request);
        }
        while let Ok(response) = self.help_responses.try_recv() {
            if let Some(response) = self.accept_discovery(response) {
                responses.push(response);
            }
        }
        while let Ok(response) = self.providers.responses.try_recv() {
            if response.query.request.generation != self.generation {
                continue;
            }
            if !response.matched {
                continue;
            }
            let mut items = self.local.complete(&response.query.request).items;
            items.retain(|item| item.source != CompletionSource::Filesystem);
            items.extend(response.items);
            responses.push(self.local_response(&response.query.request, items, response.elapsed));
        }
        while let Ok(response) = self.responses.try_recv() {
            responses.push(response);
        }
        responses.into_iter()
    }
    fn accept_discovery(&mut self, response: parser::HelpResponse) -> Option<CompletionResponse> {
        let parser::HelpResponse {
            id,
            key,
            path,
            request: response_request,
            graph,
            elapsed,
            cancelled,
        } = response;
        self.help_pending.remove(&id);
        if self.prefetch_inflight.as_deref() == Some(id.as_str()) {
            self.prefetch_inflight = None;
        }
        let latest = self.help_latest.remove(&id);
        let request = latest.clone().unwrap_or(response_request);
        if cancelled {
            if latest.is_some() {
                self.retry_current(&request, path);
            }
            self.pump_prefetch();
            return None;
        }
        let child_paths = self.local.cache_graph(key, path, graph);
        if let Some(invocation) = parser::invocation(&request) {
            for (path, discover) in child_paths {
                self.queue_prefetch(parser::request_for_path(
                    &request,
                    &invocation,
                    path,
                    discover,
                ));
            }
        }
        if request.generation != self.generation {
            self.pump_prefetch();
            return None;
        }
        let result = self.local.complete(&request);
        if let Some(help) = result.help {
            self.enqueue_help(help);
        }
        if let Some(query) = result.provider {
            self.providers.submit(query);
        }
        let items = result.items;
        let response = self.local_response(&request, items, elapsed);
        self.pump_prefetch();
        Some(response)
    }
    fn retry_current(&mut self, request: &crate::Request, path: Vec<String>) {
        if request.generation == self.generation
            && let Some(invocation) = parser::invocation(request)
        {
            self.enqueue_help(parser::request_for_path(request, &invocation, path, true));
        }
    }
}
