mod commands;
mod discovery_response;
mod engine;
mod filesystem;
mod graph;
pub mod inspection;
mod inspection_candidates;
mod literal;
mod model;
mod parser;
mod path;
mod prefetch;
mod providers;
pub mod ranking;
mod response;
mod shell;
mod token;
mod variables;

pub use model::Request;
pub use model::{
    Completion, CompletionItem, CompletionKind, CompletionResponse, CompletionResponseSource,
    CompletionSource,
};

pub fn dump_command_graph(command: &str, cwd: &Path) -> Result<String, String> {
    parser::render_command_graph(command, cwd)
}

pub fn dump_command_root(command: &str, cwd: &Path) -> Result<String, String> {
    parser::render_command_root(command, cwd)
}

pub fn dump_command_chunk(command: &str, path: &[String], cwd: &Path) -> Result<String, String> {
    parser::render_command_chunk(command, cwd, path)
}

use std::{
    collections::{HashMap, HashSet, VecDeque},
    ops::Range,
    path::Path,
    sync::mpsc::{self, Receiver, Sender},
};

pub struct CompletionEngine {
    response_tx: Sender<CompletionResponse>,
    responses: Receiver<CompletionResponse>,
    help_requests: parser::schedule::Dispatcher,
    help_responses: Receiver<parser::HelpResponse>,
    help_pending: HashSet<String>,
    help_latest: HashMap<String, Request>,
    help_prefetch: VecDeque<parser::HelpRequest>,
    prefetch_keys: HashSet<String>,
    prefetch_inflight: Option<String>,
    generation: u64,
    local: engine::LocalCompletion,
    providers: providers::Worker,
}

impl CompletionEngine {
    pub fn new() -> Option<Self> {
        let (response_tx, response_rx) = mpsc::channel();
        let help_worker = parser::Worker::spawn()?;
        Some(Self {
            response_tx,
            responses: response_rx,
            help_requests: help_worker.requests,
            help_responses: help_worker.responses,
            help_pending: HashSet::new(),
            help_latest: HashMap::new(),
            help_prefetch: VecDeque::new(),
            prefetch_keys: HashSet::new(),
            prefetch_inflight: None,
            generation: 0,
            local: engine::LocalCompletion::new(),
            providers: providers::Worker::spawn()?,
        })
    }

    /// Start a new edit without discarding discovered command graphs.
    pub fn begin_edit(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.help_requests.cancel();
        self.providers.cancel();
        self.help_latest.clear();
        self.help_prefetch.clear();
        self.prefetch_keys.clear();
        self.generation
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

impl CompletionEngine {
    pub fn provider_configuration() -> Result<String, String> {
        providers::inspect()
    }
}
