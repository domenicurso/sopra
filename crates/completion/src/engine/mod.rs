pub(crate) mod arguments;
mod candidates;
mod commands;
mod graphs;
pub(crate) mod values;

use crate::{CompletionItem, Request, filesystem::FilesystemEngine, parser, ranking, variables};

pub(super) struct LocalCompletion {
    filesystem: FilesystemEngine,
    graphs: graphs::Graphs,
}

pub(super) struct LocalResult {
    pub(super) items: Vec<CompletionItem>,
    pub(super) prefetch: bool,
    pub(super) help: Option<parser::HelpRequest>,
    pub(super) provider: Option<crate::providers::Query>,
}

impl LocalCompletion {
    pub(super) fn new() -> Self {
        Self {
            filesystem: FilesystemEngine::new(),
            graphs: graphs::Graphs::new(),
        }
    }

    pub(super) fn complete(&mut self, request: &Request) -> LocalResult {
        let cursor = ranking::byte_offset(&request.line, request.cursor);
        let (start, _) = ranking::token_range(&request.line, cursor);
        let token = &request.line[start..cursor];
        if let Some(items) = variables::complete(request, request.replace.clone(), token) {
            return handled(items, false, None);
        }
        if is_path_context(token) {
            return handled(self.filesystem.complete(request, token), false, None);
        }
        if let Some(invocation) = parser::invocation(request)
            && let Some(mut result) = self.complete_command(request, &invocation)
        {
            if result.items.is_empty() && !token.starts_with('-') {
                result.items = self.filesystem.complete(request, token);
            }
            return result;
        }
        if crate::shell::command_position(&request.line, cursor) {
            return handled(
                crate::commands::complete(token, request.replace.clone()),
                true,
                None,
            );
        }
        let items = if token.starts_with('-') {
            Vec::new()
        } else {
            self.filesystem.complete(request, token)
        };
        handled(items, false, None)
    }

    fn complete_command(
        &mut self,
        request: &Request,
        invocation: &parser::Invocation,
    ) -> Option<LocalResult> {
        self.restore(request, invocation);
        let Some(entry) = self.graphs.get(&invocation.key) else {
            return Some(handled(
                Vec::new(),
                false,
                Some(parser::request_for(request, invocation)),
            ));
        };
        let cached = &entry.graph;
        let Some(graph) = cached else {
            return Some(handled(Vec::new(), false, None));
        };
        let (node, scoped, path) = parser::resolve(graph, invocation);
        let help = if entry.stale() {
            let mut help = parser::request_for(request, invocation);
            help.cache = parser::discover::CachePolicy::Refresh;
            Some(help)
        } else if path.is_empty() || entry.hydrated.contains(&path) {
            None
        } else {
            let discover = node.subcommands.is_empty() && node.positionals.is_empty();
            Some(parser::request_for_path(
                request,
                invocation,
                path.clone(),
                discover,
            ))
        };
        let mut result = handled(
            candidates::complete(node, &scoped, request, &mut self.filesystem),
            false,
            help,
        );
        result.provider = crate::providers::plan(request, &scoped, path, node);
        Some(result)
    }

    fn restore(&mut self, request: &Request, invocation: &parser::Invocation) {
        if self.graphs.get(&invocation.key).is_none() {
            let help = parser::request_for(request, invocation);
            if let Some(hit) = parser::snapshot::read(&help) {
                self.graphs
                    .merge(invocation.key.clone(), Vec::new(), Some(hit.graph));
                if hit.stale {
                    self.graphs.provisional(&invocation.key);
                }
            }
        }
        let Some(entry) = self.graphs.get(&invocation.key) else {
            return;
        };
        let Some(graph) = &entry.graph else {
            return;
        };
        let (_, _, path) = parser::resolve(graph, invocation);
        if path.is_empty() || entry.hydrated.contains(&path) {
            return;
        }
        let help = parser::request_for_path(request, invocation, path.clone(), true);
        if let Some(hit) = parser::snapshot::read(&help) {
            self.graphs
                .merge(invocation.key.clone(), path, Some(hit.graph));
            if hit.stale {
                self.graphs.provisional(&invocation.key);
            }
        }
    }

    pub(super) fn cache_graph(
        &mut self,
        key: String,
        path: Vec<String>,
        graph: Option<crate::graph::CommandGraph>,
    ) -> Vec<(Vec<String>, bool)> {
        self.graphs.merge(key, path, graph)
    }

    pub(super) fn needs_graph(&mut self, key: &str) -> bool {
        self.graphs.get(key).is_none()
    }
}

fn handled(
    items: Vec<CompletionItem>,
    prefetch: bool,
    help: Option<parser::HelpRequest>,
) -> LocalResult {
    LocalResult {
        items,
        prefetch,
        help,
        provider: None,
    }
}

fn is_path_context(token: &str) -> bool {
    token.starts_with(['/', '.', '~']) || token.contains('/')
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod value_tests;

#[cfg(test)]
mod grammar_tests;
