use std::collections::HashMap;

use super::{CompletionItem, Request, filesystem::FilesystemEngine, help, ranking, variables};

pub(super) struct LocalCompletion {
    filesystem: FilesystemEngine,
    help: HashMap<String, Option<help::HelpSpec>>,
}

pub(super) struct LocalResult {
    pub(super) items: Vec<CompletionItem>,
    pub(super) prefetch: bool,
    pub(super) help: Option<help::HelpRequest>,
}

impl LocalCompletion {
    pub(super) fn new() -> Self {
        Self {
            filesystem: FilesystemEngine::new(),
            help: HashMap::new(),
        }
    }

    pub(super) fn complete(&mut self, request: &Request) -> LocalResult {
        let cursor = ranking::byte_offset(&request.line, request.cursor);
        let (start, _) = ranking::token_range(&request.line, cursor);
        let token = &request.line[start..cursor];
        if let Some(items) = variables::complete(request, request.replace.clone(), token) {
            return LocalResult {
                items,
                prefetch: false,
                help: None,
            };
        }
        if is_path_context(&request.line[..start], token) {
            return LocalResult {
                items: self.filesystem.complete(request, token),
                prefetch: false,
                help: None,
            };
        }
        if let Some(invocation) = help::invocation(request)
            && let Some(result) = self.complete_help(request, &invocation)
        {
            return result;
        }
        if crate::shell::command_position(&request.line, cursor) {
            return LocalResult {
                items: super::commands::complete(token, request.replace.clone()),
                prefetch: true,
                help: None,
            };
        }
        LocalResult {
            items: Vec::new(),
            prefetch: false,
            help: None,
        }
    }

    fn complete_help(
        &mut self,
        request: &Request,
        invocation: &help::Invocation,
    ) -> Option<LocalResult> {
        let Some(cached) = self.help.get(&invocation.key).cloned() else {
            return Some(LocalResult {
                items: Vec::new(),
                prefetch: false,
                help: Some(help::request_for(request, invocation)),
            });
        };
        let Some(spec) = cached else {
            return Some(handled(Vec::new(), None));
        };
        if let Some((nested_request, nested_invocation)) =
            help::nested_request(request, invocation, &spec)
        {
            match self.help.get(&nested_request.key).cloned() {
                Some(Some(nested_spec)) => {
                    let items = help::complete(
                        &nested_spec,
                        &nested_invocation,
                        request,
                        &mut self.filesystem,
                    );
                    return Some(handled(items, None));
                }
                Some(None) => return Some(handled(Vec::new(), None)),
                None => {
                    let items = help::complete(&spec, invocation, request, &mut self.filesystem);
                    return Some(handled(items, Some(nested_request)));
                }
            }
        }
        let items = help::complete(&spec, invocation, request, &mut self.filesystem);
        Some(handled(items, None))
    }

    pub(super) fn cache_help(&mut self, key: String, spec: Option<help::HelpSpec>) {
        self.help.insert(key, spec);
    }

    pub(super) fn needs_help(&self, key: &str) -> bool {
        !self.help.contains_key(key)
    }
}

fn handled(items: Vec<CompletionItem>, help: Option<help::HelpRequest>) -> LocalResult {
    LocalResult {
        items,
        prefetch: false,
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

    use super::{LocalCompletion, Request};

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
