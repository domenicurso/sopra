mod commands;
mod filesystem;
mod local;
mod model;
mod path;
mod prefetch;
pub(crate) mod ranking;
mod worker;

pub(crate) use model::Request;
pub(crate) use model::{
    Completion, CompletionItem, CompletionKind, CompletionResponse, CompletionResponseSource,
    CompletionSource,
};

use std::{
    collections::HashMap,
    ops::Range,
    path::Path,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

struct CachedCompletion {
    items: Vec<CompletionItem>,
    elapsed: Duration,
}

pub(crate) struct CompletionEngine {
    requests: Sender<Request>,
    response_tx: Sender<CompletionResponse>,
    responses: Receiver<CompletionResponse>,
    generation: u64,
    pending_context: Option<String>,
    cache: HashMap<String, CachedCompletion>,
    local: local::LocalCompletion,
}

impl CompletionEngine {
    pub(crate) fn new() -> Option<Self> {
        let (request_tx, request_rx) = mpsc::channel();
        let (response_tx, response_rx) = mpsc::channel();
        let worker_response_tx = response_tx.clone();
        thread::Builder::new()
            .name("keel-zshrs-completion".to_string())
            .spawn(move || worker::run(request_rx, worker_response_tx))
            .ok()?;
        Some(Self {
            requests: request_tx,
            response_tx,
            responses: response_rx,
            generation: 0,
            pending_context: None,
            cache: HashMap::new(),
            local: local::LocalCompletion::new(),
        })
    }

    pub(crate) fn request(&mut self, line: &str, cursor: usize, cwd: &Path) {
        self.generation = self.generation.wrapping_add(1);
        let (context_line, context_cursor) = ranking::broad_context(line, cursor);
        let replace = active_range(line, cursor);
        let context_key = format!("{}\0{}", cwd.display(), context_line);
        let request = Request {
            line: line.to_string(),
            cursor,
            context_line,
            context_cursor,
            replace,
            cwd: cwd.to_path_buf(),
            generation: self.generation,
            context_key: context_key.clone(),
        };
        self.send_local(&request);
        if line.trim().is_empty() {
            return;
        }
        if self.prefetch_argument_context(line, cursor, cwd) {
            return;
        }
        if let Some(cached) = self.cache.get(&context_key) {
            let items = rebind(&cached.items, request.replace.clone());
            self.send_zshrs(&request, items, cached.elapsed);
            return;
        }
        if self.pending_context.as_deref() == Some(context_key.as_str()) {
            return;
        }
        self.pending_context = Some(context_key);
        let _ = self.requests.send(request);
    }

    pub(crate) fn poll(&mut self) -> impl Iterator<Item = CompletionResponse> {
        let mut responses = Vec::new();
        while let Ok(response) = self.responses.try_recv() {
            if response.source == CompletionResponseSource::Zshrs {
                if self.pending_context.as_deref() == Some(response.context_key.as_str()) {
                    self.pending_context = None;
                }
                if !response.incomplete && !response.items.is_empty() {
                    self.cache.insert(
                        response.context_key.clone(),
                        CachedCompletion {
                            items: response.items.clone(),
                            elapsed: response.elapsed,
                        },
                    );
                }
            }
            responses.push(response);
        }
        responses.into_iter()
    }

    fn send_local(&mut self, request: &Request) {
        let started = Instant::now();
        let items = self.local.complete(request);
        let _ = self.response_tx.send(CompletionResponse {
            line: request.line.clone(),
            cursor: request.cursor,
            context_line: request.context_line.clone(),
            context_cursor: request.context_cursor,
            context_key: request.context_key.clone(),
            items,
            generation: request.generation,
            source: CompletionResponseSource::Local,
            incomplete: false,
            elapsed: started.elapsed(),
        });
    }

    fn send_zshrs(&self, request: &Request, items: Vec<CompletionItem>, elapsed: Duration) {
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

fn active_range(line: &str, cursor: usize) -> Range<usize> {
    let (start, end) = ranking::token_range(line, ranking::byte_offset(line, cursor));
    start..end
}

pub(crate) fn rebind(items: &[CompletionItem], replace: Range<usize>) -> Vec<CompletionItem> {
    items
        .iter()
        .cloned()
        .map(|mut item| {
            item.replace = replace.clone();
            item
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::CompletionEngine;

    #[test]
    fn local_completion_refreshes_inside_a_cached_semantic_context() {
        let mut engine = CompletionEngine::new().expect("completion worker");
        let cwd = PathBuf::from(".");
        engine.request("cd ", 3, &cwd);
        engine.request("cd C", 4, &cwd);
        let responses = engine.poll().collect::<Vec<_>>();
        assert_eq!(
            responses
                .iter()
                .filter(|response| response.source == super::CompletionResponseSource::Local)
                .count(),
            2
        );
    }
}
