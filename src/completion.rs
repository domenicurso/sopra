mod commands;
mod filesystem;
mod help;
mod local;
mod model;
mod path;
mod prefetch;
pub(crate) mod ranking;
mod response;
mod token;
mod variables;
mod worker;

pub(crate) use model::Request;
pub(crate) use model::{
    Completion, CompletionItem, CompletionKind, CompletionResponse, CompletionResponseSource,
    CompletionSource,
};

use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    path::Path,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Duration,
};

struct CachedCompletion {
    items: Vec<CompletionItem>,
    elapsed: Duration,
}

pub(crate) struct CompletionEngine {
    requests: Sender<Request>,
    response_tx: Sender<CompletionResponse>,
    responses: Receiver<CompletionResponse>,
    help_requests: Sender<help::HelpRequest>,
    help_responses: Receiver<help::HelpResponse>,
    help_pending: HashSet<String>,
    help_latest: HashMap<String, Request>,
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
        let help_worker = help::Worker::spawn()?;
        Some(Self {
            requests: request_tx,
            response_tx,
            responses: response_rx,
            help_requests: help_worker.requests,
            help_responses: help_worker.responses,
            help_pending: HashSet::new(),
            help_latest: HashMap::new(),
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
        if line.trim().is_empty() {
            return;
        }
        let local = self.send_local(&request);
        if local.prefetch && self.prefetch_argument_context(line, cursor, cwd) {
            return;
        }
        if local.handled {
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
        while let Ok(response) = self.help_responses.try_recv() {
            let help::HelpResponse {
                key,
                request: response_request,
                spec,
                elapsed,
            } = response;
            self.help_pending.remove(&key);
            let request = self.help_latest.remove(&key).unwrap_or(response_request);
            self.local.cache_help(key, spec);
            let items = self.local.complete(&request).items;
            responses.push(self.local_response(&request, items, elapsed));
        }
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
mod tests;
