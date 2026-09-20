use std::{
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

use crate::{Request, graph::CommandGraph};

use super::{
    cache::{self, Source},
    expand, help, man, process,
};

#[derive(Debug, Clone)]
pub(crate) struct HelpRequest {
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
        let request = drain_latest(request, &requests);
        let started = Instant::now();
        let graph = resolve_chunk(&request);
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

fn drain_latest(mut request: HelpRequest, requests: &Receiver<HelpRequest>) -> HelpRequest {
    while let Ok(next) = requests.try_recv() {
        request = next;
    }
    request
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
    if let Some(sources) = cache::read(request)
        && let Some(mut graph) = aggregate(request, &sources, discover_usage_commands)
        && graph.useful()
    {
        graph.sort();
        return Some(graph);
    }
    let sources = probe_sources(request);
    let mut graph = aggregate(request, &sources, discover_usage_commands)?;
    if !graph.useful() {
        return None;
    }
    graph.sort();
    cache::write(request, &sources);
    Some(graph)
}

fn probe_sources(request: &HelpRequest) -> Vec<Source> {
    let mut probes = Vec::new();
    for args in help_arguments(request) {
        let request = request.clone();
        probes.push(thread::spawn(move || {
            run_program(&request, &args).map(Source::Help)
        }));
    }
    let request = request.clone();
    probes.push(thread::spawn(move || run_man(&request).map(Source::Man)));
    probes
        .into_iter()
        .filter_map(|probe| probe.join().ok().flatten())
        .collect()
}

fn help_arguments(request: &HelpRequest) -> Vec<Vec<String>> {
    ["--help", "-h", "help"]
        .iter()
        .map(|probe| {
            let mut args = request.args.clone();
            args.push((*probe).to_string());
            args
        })
        .chain((!request.args.is_empty()).then(|| {
            let mut args = vec!["help".to_string()];
            args.extend(request.args.iter().cloned());
            args
        }))
        .collect()
}

fn aggregate(
    request: &HelpRequest,
    sources: &[Source],
    discover_usage_commands: bool,
) -> Option<CommandGraph> {
    let mut graph: Option<CommandGraph> = None;
    for source in sources {
        let fragment = match source {
            Source::Help(text) if help::matches_path(text, &request.program, &request.args) => {
                help::parse_for_with(
                    text,
                    &request.program,
                    &request.args,
                    discover_usage_commands,
                )
            }
            Source::Help(_) => continue,
            Source::Man(text) => man::parse_for_with(
                text,
                &request.program,
                &request.args,
                discover_usage_commands,
            ),
        };
        if !fragment.root.useful() {
            continue;
        }
        if let Some(current) = &mut graph {
            current.merge(fragment);
        } else {
            graph = Some(fragment);
        }
    }
    graph
}

fn run_program(request: &HelpRequest, args: &[String]) -> Option<String> {
    let program = request
        .command
        .as_deref()
        .unwrap_or_else(|| std::path::Path::new(&request.program));
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(&request.cwd)
        .env("TERM", "dumb")
        .env("NO_COLOR", "1")
        .env("PAGER", "cat")
        .env("GIT_PAGER", "cat")
        .env("GIT_MAN_VIEWER", "cat")
        .env("MANPAGER", "cat")
        .env("LC_ALL", "C");
    process::run(&mut command)
}

fn run_man(request: &HelpRequest) -> Option<String> {
    let title = std::iter::once(request.program.as_str())
        .chain(request.args.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join("-");
    let mut command = Command::new("man");
    command
        .args(["-P", "cat", &title])
        .current_dir(&request.cwd)
        .env("MANPAGER", "cat")
        .env("PAGER", "cat")
        .stdin(Stdio::null());
    process::run(&mut command)
}
