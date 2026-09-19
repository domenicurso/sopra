mod command_catalog;
mod command_discovery;
mod commands;
mod filesystem;
mod help;
mod local;
mod model;
mod path;
mod prefetch;
pub mod ranking;
mod response;
mod shell;
mod token;
mod variables;

pub use commands::command_available;
pub use model::Request;
pub use model::{
    Completion, CompletionItem, CompletionKind, CompletionResponse, CompletionResponseSource,
    CompletionSource,
};

use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    path::Path,
    sync::mpsc::{self, Receiver, Sender},
};

pub struct CompletionEngine {
    response_tx: Sender<CompletionResponse>,
    responses: Receiver<CompletionResponse>,
    help_requests: Sender<help::HelpRequest>,
    help_responses: Receiver<help::HelpResponse>,
    help_pending: HashSet<String>,
    help_latest: HashMap<String, Request>,
    generation: u64,
    local: local::LocalCompletion,
}

impl CompletionEngine {
    pub fn new() -> Option<Self> {
        let (response_tx, response_rx) = mpsc::channel();
        let help_worker = help::Worker::spawn()?;
        Some(Self {
            response_tx,
            responses: response_rx,
            help_requests: help_worker.requests,
            help_responses: help_worker.responses,
            help_pending: HashSet::new(),
            help_latest: HashMap::new(),
            generation: 0,
            local: local::LocalCompletion::new(),
        })
    }

    pub fn request(&mut self, line: &str, cursor: usize, cwd: &Path) {
        self.generation = self.generation.wrapping_add(1);
        let (context_line, context_cursor) = ranking::broad_context(line, cursor);
        let replace = active_range(line, cursor);
        let request = Request {
            line: line.to_string(),
            cursor,
            context_line,
            context_cursor,
            replace,
            cwd: cwd.to_path_buf(),
            generation: self.generation,
            context_key: format!("{}\0{}", cwd.display(), line),
        };
        if line.trim().is_empty() {
            return;
        }
        if self.send_local(&request) {
            self.prefetch_argument_context(line, cursor, cwd);
        }
    }

    pub fn poll(&mut self) -> impl Iterator<Item = CompletionResponse> {
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
            responses.push(response);
        }
        responses.into_iter()
    }
}

fn active_range(line: &str, cursor: usize) -> Range<usize> {
    let (start, end) = ranking::token_range(line, ranking::byte_offset(line, cursor));
    start..end
}

pub fn rebind(items: &[CompletionItem], replace: Range<usize>) -> Vec<CompletionItem> {
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
