use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, Sender},
};

use super::{CompletionResponse, Request, pty};

pub(super) fn run(
    provider: Option<PathBuf>,
    requests: Receiver<Request>,
    responses: Sender<CompletionResponse>,
) {
    while let Ok(mut request) = requests.recv() {
        while let Ok(next) = requests.try_recv() {
            request = next;
        }
        let Some(provider) = provider.as_deref() else {
            continue;
        };
        let items = pty::capture(provider, &request).unwrap_or_default();
        if !send_response(&responses, &request, items) {
            return;
        }
    }
}

fn send_response(
    responses: &Sender<CompletionResponse>,
    request: &Request,
    items: Vec<super::CompletionItem>,
) -> bool {
    responses
        .send(CompletionResponse {
            line: request.line.clone(),
            cursor: request.cursor,
            context_line: request.provider_line.clone(),
            context_cursor: request.provider_cursor,
            items,
            generation: request.generation,
            provider: true,
        })
        .is_ok()
}
