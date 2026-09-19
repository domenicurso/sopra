use std::collections::HashSet;

use crate::graph::CommandGraph;

use super::discover::HelpRequest;

const MAX_DEPTH: usize = 8;
const MAX_NODES: usize = 512;
const BATCH_SIZE: usize = 8;

pub(super) fn resolve<F>(request: &HelpRequest, mut graph: CommandGraph, load: F) -> CommandGraph
where
    F: Fn(&HelpRequest, bool) -> Option<CommandGraph> + Send + Sync,
{
    let mut queue = initial_paths(&graph);
    let mut visited = HashSet::new();
    while !queue.is_empty() && visited.len() < MAX_NODES {
        let work = prepare_batch(&mut queue, &mut visited, request, &graph);
        let loader = &load;
        let results = std::thread::scope(|scope| {
            work.iter()
                .map(|(path, nested, discover)| {
                    scope.spawn(move || {
                        loader(nested, *discover).map(|fragment| (path.clone(), fragment))
                    })
                })
                .collect::<Vec<_>>()
                .into_iter()
                .filter_map(|task| task.join().ok().flatten())
                .collect::<Vec<_>>()
        });
        for (path, fragment) in results {
            if !graph.merge_at(&path, fragment.root) {
                continue;
            }
            if let Some(node) = graph.root.find(&path) {
                enqueue_children(&mut queue, &path, node);
            }
        }
    }
    graph.sort();
    graph
}

type WorkItem = (Vec<String>, HelpRequest, bool);

fn prepare_batch(
    queue: &mut Vec<Vec<String>>,
    visited: &mut HashSet<Vec<String>>,
    request: &HelpRequest,
    graph: &CommandGraph,
) -> Vec<WorkItem> {
    let mut work = Vec::new();
    while work.len() < BATCH_SIZE {
        let Some(path) = queue.pop() else {
            break;
        };
        if path.len() > MAX_DEPTH || !visited.insert(path.clone()) {
            continue;
        }
        let Some(node) = graph.root.find(&path) else {
            continue;
        };
        let discover = node.subcommands.is_empty() && node.positionals.is_empty();
        work.push((path.clone(), request_at_path(request, &path), discover));
    }
    work
}

fn initial_paths(graph: &CommandGraph) -> Vec<Vec<String>> {
    let mut queue = Vec::new();
    enqueue_children(&mut queue, &[], &graph.root);
    queue
}

fn enqueue_children(
    queue: &mut Vec<Vec<String>>,
    prefix: &[String],
    node: &crate::graph::CommandNode,
) {
    for child in &node.subcommands {
        let mut path = prefix.to_vec();
        path.push(child.name.clone());
        queue.push(path);
    }
}

fn request_at_path(request: &HelpRequest, path: &[String]) -> HelpRequest {
    let mut nested = request.clone();
    nested.args = path.to_vec();
    nested
}
