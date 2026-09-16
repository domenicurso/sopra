mod commands;
mod local;
mod model;
mod path;
mod provider;
mod pty;
pub(crate) mod ranking;

pub(crate) use model::{CompletionItem, CompletionKind};

use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

#[derive(Debug, Clone)]
struct Request {
    line: String,
    cursor: usize,
    provider_line: String,
    provider_cursor: usize,
    cwd: PathBuf,
    generation: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct CompletionResponse {
    pub(crate) line: String,
    pub(crate) cursor: usize,
    pub(crate) context_line: String,
    pub(crate) context_cursor: usize,
    pub(crate) items: Vec<CompletionItem>,
    pub(crate) generation: u64,
    pub(crate) provider: bool,
}

pub(crate) struct CompletionClient {
    requests: Sender<Request>,
    response_tx: Sender<CompletionResponse>,
    responses: Receiver<CompletionResponse>,
    generation: u64,
    provider_enabled: bool,
    provider_context: Option<(String, usize)>,
}

impl CompletionClient {
    pub(crate) fn new(provider: Option<PathBuf>) -> Option<Self> {
        let provider = provider.filter(|path| path.is_file());
        let provider_enabled = provider.is_some();
        let (request_tx, request_rx) = mpsc::channel();
        let (response_tx, response_rx) = mpsc::channel();
        let worker_response_tx = response_tx.clone();
        thread::Builder::new()
            .name("keel-completion".to_string())
            .spawn(move || provider::run(provider, request_rx, worker_response_tx))
            .ok()?;
        Some(Self {
            requests: request_tx,
            response_tx: response_tx.clone(),
            responses: response_rx,
            generation: 0,
            provider_enabled,
            provider_context: None,
        })
    }

    pub(crate) fn request(&mut self, line: &str, cursor: usize, cwd: &Path) {
        let (provider_line, provider_cursor) = ranking::broad_context(line, cursor);
        self.generation = self.generation.wrapping_add(1);
        let request = Request {
            line: line.to_string(),
            cursor,
            provider_line,
            provider_cursor,
            cwd: cwd.to_path_buf(),
            generation: self.generation,
        };
        let local_items = local::complete(&request);
        let _ = self.response_tx.send(CompletionResponse {
            line: request.line.clone(),
            cursor: request.cursor,
            context_line: request.provider_line.clone(),
            context_cursor: request.provider_cursor,
            items: local_items,
            generation: request.generation,
            provider: false,
        });
        let context = (request.provider_line.clone(), request.provider_cursor);
        if !self.provider_enabled {
            return;
        }
        if request.line.trim().is_empty() {
            self.provider_context = None;
            return;
        }
        if self.provider_context.as_ref() == Some(&context) {
            return;
        }
        self.provider_context = Some(context);
        let _ = self.requests.send(request);
    }

    pub(crate) fn poll(&self) -> impl Iterator<Item = CompletionResponse> {
        let mut responses = Vec::new();
        while let Ok(response) = self.responses.try_recv() {
            responses.push(response);
        }
        responses.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::CompletionClient;

    #[test]
    fn local_completion_refreshes_inside_a_cached_provider_context() {
        let mut client = CompletionClient::new(None).expect("completion worker");
        let cwd = PathBuf::from(".");
        client.request("cd ", 3, &cwd);
        client.request("cd C", 4, &cwd);
        let responses = client.poll().collect::<Vec<_>>();
        assert_eq!(
            responses
                .iter()
                .filter(|response| !response.provider)
                .count(),
            2
        );
        assert!(
            responses
                .iter()
                .any(|response| response.line == "cd C" && response.cursor == 4)
        );
    }
}
