#[path = "probes.rs"]
pub(super) mod probes;
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

use crate::{Request, graph::CommandGraph};

use super::aggregate::aggregate;
use super::{cache, expand};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CachePolicy {
    Use,
    Refresh,
}

#[derive(Debug, Clone)]
pub(crate) struct HelpRequest {
    pub(crate) cache: CachePolicy,
    pub(crate) environment: sopra_shell_context::Snapshot,
    pub(crate) key: String,
    pub(crate) command: Option<PathBuf>,
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) cwd: PathBuf,
    pub(crate) request: Request,
    pub(crate) discover_usage_commands: bool,
}

#[derive(Debug)]
pub(crate) struct HelpResponse {
    pub(crate) id: String,
    pub(crate) key: String,
    pub(crate) path: Vec<String>,
    pub(crate) request: Request,
    pub(crate) graph: Option<CommandGraph>,
    pub(crate) elapsed: Duration,
}

pub(crate) struct Worker {
    pub(crate) requests: Sender<HelpRequest>,
    pub(crate) responses: Receiver<HelpResponse>,
}

impl Worker {
    pub(crate) fn spawn() -> Option<Self> {
        let (requests, request_rx) = mpsc::channel();
        let (responses, response_rx) = mpsc::channel();
        thread::Builder::new()
            .name("completion-discovery".to_string())
            .spawn(move || run(request_rx, responses))
            .ok()?;
        Some(Self {
            requests,
            responses: response_rx,
        })
    }
}

impl HelpRequest {
    pub(crate) fn id(&self) -> String {
        if self.args.is_empty() {
            return self.key.clone();
        }
        format!("{}\0{}", self.key, self.args.join("\0"))
    }
}

fn run(requests: Receiver<HelpRequest>, responses: Sender<HelpResponse>) {
    while let Ok(request) = requests.recv() {
        let started = Instant::now();
        let graph = request.environment.with(|| resolve_chunk(&request));
        if responses
            .send(HelpResponse {
                id: request.id(),
                key: request.key.clone(),
                path: request.args.clone(),
                request: request.request.clone(),
                graph,
                elapsed: started.elapsed(),
            })
            .is_err()
        {
            break;
        }
    }
}

pub(super) fn resolve(request: &HelpRequest) -> Option<CommandGraph> {
    let graph = fragment(request, true)?;
    Some(expand::resolve(request, graph, |nested, discover| {
        fragment(nested, discover)
    }))
}

pub(super) fn resolve_chunk(request: &HelpRequest) -> Option<CommandGraph> {
    fragment(request, request.discover_usage_commands)
}

fn fragment(request: &HelpRequest, discover_usage_commands: bool) -> Option<CommandGraph> {
    if request.cache == CachePolicy::Use
        && let Some(sources) = cache::read(request)
        && let Some(mut graph) = aggregate(request, &sources, discover_usage_commands)
        && graph.useful()
    {
        graph.sort();
        return Some(graph);
    }
    let sources = probes::sources(request);
    let mut graph = aggregate(request, &sources, discover_usage_commands)?;
    if !graph.useful() {
        return None;
    }
    graph.sort();
    cache::write(request, &sources);
    Some(graph)
}
