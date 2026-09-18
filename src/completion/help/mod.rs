mod complete;
mod context;
mod discover;
mod model;
mod output;
mod parse;
mod parse_commands;
mod parse_rows;
mod parse_values;
mod target;
mod usage;
mod worker;

use std::sync::mpsc::{self, Receiver, Sender};

use super::{CompletionItem, Request, filesystem::FilesystemEngine};

pub(super) use context::Invocation;
pub(super) use model::{HelpRequest, HelpResponse, HelpSpec};
#[cfg(test)]
pub(super) use parse::parse_help;
#[cfg(test)]
mod tests;

pub(super) struct Worker {
    pub(super) requests: Sender<HelpRequest>,
    pub(super) responses: Receiver<HelpResponse>,
}

impl Worker {
    pub(super) fn spawn() -> Option<Self> {
        let (requests, request_rx) = mpsc::channel();
        let (responses, response_rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("keel-command-help".to_string())
            .spawn(move || worker::run(request_rx, responses))
            .ok()?;
        Some(Self {
            requests,
            responses: response_rx,
        })
    }
}

pub(super) fn invocation(request: &Request) -> Option<Invocation> {
    context::invocation(request)
}

pub(super) fn request_for(request: &Request, invocation: &Invocation) -> HelpRequest {
    context::request_for(request, invocation)
}

pub(super) fn nested_request(
    request: &Request,
    invocation: &Invocation,
    spec: &HelpSpec,
) -> Option<(HelpRequest, Invocation)> {
    target::nested_request(request, invocation, spec)
}

pub(super) fn complete(
    spec: &HelpSpec,
    invocation: &Invocation,
    request: &Request,
    filesystem: &mut FilesystemEngine,
) -> Vec<CompletionItem> {
    complete::complete(spec, invocation, request, filesystem)
}
