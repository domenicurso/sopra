#[path = "probes.rs"]
pub(super) mod probes;
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    time::Duration,
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
    pub(crate) control: super::control::Control,
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
    pub(crate) cancelled: bool,
}

pub(crate) struct Worker {
    pub(crate) requests: super::schedule::Dispatcher,
    pub(crate) responses: Receiver<HelpResponse>,
}

impl Worker {
    pub(crate) fn spawn() -> Option<Self> {
        let (responses, response_rx) = mpsc::channel();
        let requests = super::schedule::Dispatcher::spawn(responses)?;
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

pub(crate) fn resolve(request: &HelpRequest) -> Option<CommandGraph> {
    let graph = fragment(request, true)?;
    Some(expand::resolve(request, graph, |nested, discover| {
        fragment(nested, discover)
    }))
}

pub(crate) fn resolve_chunk(request: &HelpRequest) -> Option<CommandGraph> {
    fragment(request, request.discover_usage_commands)
}

fn fragment(request: &HelpRequest, discover_usage_commands: bool) -> Option<CommandGraph> {
    if request.control.stopped() {
        return None;
    }
    if request.cache == CachePolicy::Use
        && let Some(sources) = cache::read(request)
        && let Some(mut graph) = aggregate(request, &sources, discover_usage_commands)
        && graph.useful()
    {
        graph.sort();
        super::snapshot::write(request, &graph);
        return Some(graph);
    }
    let sources = probes::sources(request);
    if request.control.stopped() {
        return None;
    }
    let mut graph = aggregate(request, &sources, discover_usage_commands)?;
    if !graph.useful() {
        return None;
    }
    graph.sort();
    cache::write(request, &sources);
    super::snapshot::write(request, &graph);
    Some(graph)
}
