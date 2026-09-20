mod arguments;
mod candidates;

use std::collections::{HashMap, HashSet};

use crate::{CompletionItem, Request, filesystem::FilesystemEngine, parser, ranking, variables};

pub(super) struct LocalCompletion {
    filesystem: FilesystemEngine,
    graphs: HashMap<String, Option<crate::graph::CommandGraph>>,
    hydrated: HashSet<String>,
}

pub(super) struct LocalResult {
    pub(super) items: Vec<CompletionItem>,
    pub(super) prefetch: bool,
    pub(super) help: Option<parser::HelpRequest>,
}

impl LocalCompletion {
    pub(super) fn new() -> Self {
        Self {
            filesystem: FilesystemEngine::new(),
            graphs: HashMap::new(),
            hydrated: HashSet::new(),
        }
    }

    pub(super) fn complete(&mut self, request: &Request) -> LocalResult {
        let cursor = ranking::byte_offset(&request.line, request.cursor);
        let (start, _) = ranking::token_range(&request.line, cursor);
        let token = &request.line[start..cursor];
        if let Some(items) = variables::complete(request, request.replace.clone(), token) {
            return handled(items, false, None);
        }
        if is_path_context(&request.line[..start], token) {
            return handled(self.filesystem.complete(request, token), false, None);
        }
        if let Some(invocation) = parser::invocation(request)
            && let Some(result) = self.complete_command(request, &invocation)
        {
            return result;
        }
        if crate::shell::command_position(&request.line, cursor) {
            return handled(
                crate::commands::complete(token, request.replace.clone()),
                true,
                None,
            );
        }
        handled(Vec::new(), false, None)
    }

    fn complete_command(
        &mut self,
        request: &Request,
        invocation: &parser::Invocation,
    ) -> Option<LocalResult> {
        let Some(cached) = self.graphs.get(&invocation.key).cloned() else {
            return Some(handled(
                Vec::new(),
                false,
                Some(parser::request_for(request, invocation)),
            ));
        };
        let Some(graph) = cached else {
            return Some(handled(Vec::new(), false, None));
        };
        let (node, scoped, path) = parser::resolve(&graph, invocation);
        let help = if path.is_empty() || self.hydrated.contains(&chunk_key(&invocation.key, &path))
        {
            None
        } else {
            let discover = node.subcommands.is_empty() && node.positionals.is_empty();
            Some(parser::request_for_path(
                request, invocation, path, discover,
            ))
        };
        Some(handled(
            candidates::complete(node, &scoped, request, &mut self.filesystem),
            false,
            help,
        ))
    }

    pub(super) fn cache_graph(
        &mut self,
        key: String,
        path: Vec<String>,
        graph: Option<crate::graph::CommandGraph>,
    ) -> Vec<(Vec<String>, bool)> {
        self.hydrated.insert(chunk_key(&key, &path));
        if path.is_empty() {
            self.graphs.insert(key.clone(), graph);
        } else {
            let Some(Some(root)) = self.graphs.get_mut(&key) else {
                return Vec::new();
            };
            if let Some(graph) = graph {
                root.merge_at(&path, graph.root);
            }
        }
        let Some(Some(root)) = self.graphs.get(&key) else {
            return Vec::new();
        };
        let Some(node) = root.root.find(&path) else {
            return Vec::new();
        };
        node.subcommands
            .iter()
            .map(|child| {
                let mut child_path = path.clone();
                child_path.push(child.name.clone());
                let discover = child.subcommands.is_empty() && child.positionals.is_empty();
                (child_path, discover)
            })
            .filter(|(child_path, _)| !self.hydrated.contains(&chunk_key(&key, child_path)))
            .collect()
    }

    pub(super) fn needs_graph(&self, key: &str) -> bool {
        !self.graphs.contains_key(key)
    }
}

fn chunk_key(root: &str, path: &[String]) -> String {
    if path.is_empty() {
        return root.to_string();
    }
    format!("{root}\0{}", path.join("\0"))
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
    }
}

fn is_path_context(prefix: &str, token: &str) -> bool {
    if token.starts_with(['/', '.', '~']) || token.contains('/') {
        return true;
    }
    let command = prefix
        .rsplit(['|', '&', ';', '\n'])
        .next()
        .and_then(|segment| segment.split_whitespace().next())
        .unwrap_or_default();
    matches!(
        command,
        "cd" | "chdir"
            | "pushd"
            | "dirs"
            | "ls"
            | "la"
            | "ll"
            | "cat"
            | "less"
            | "more"
            | "head"
            | "tail"
            | "rm"
            | "mv"
            | "cp"
            | "open"
            | "code"
            | "vim"
            | "nvim"
            | "nano"
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::LocalCompletion;
    use crate::Request;

    #[test]
    fn filesystem_completion_uses_the_explicit_token_range() {
        let mut local = LocalCompletion::new();
        let request = Request {
            line: "cd ./".to_string(),
            cursor: 5,
            context_line: "cd ./".to_string(),
            context_cursor: 5,
            replace: 3..5,
            cwd: PathBuf::from("."),
            generation: 0,
            context_key: String::new(),
        };
        for item in local.complete(&request).items {
            assert_eq!(item.replace, 3..5);
        }
    }
}
