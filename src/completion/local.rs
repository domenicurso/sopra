use super::{CompletionItem, Request, filesystem::FilesystemEngine, ranking};

pub(super) struct LocalCompletion {
    filesystem: FilesystemEngine,
}

impl LocalCompletion {
    pub(super) fn new() -> Self {
        Self {
            filesystem: FilesystemEngine::new(),
        }
    }

    pub(super) fn complete(&mut self, request: &Request) -> Vec<CompletionItem> {
        let cursor = ranking::byte_offset(&request.line, request.cursor);
        let (start, _) = ranking::token_range(&request.line, cursor);
        let token = &request.line[start..cursor];
        if is_path_context(&request.line[..start], token) {
            return self.filesystem.complete(request, token);
        }
        if crate::syntax::command_position(&request.line, cursor) {
            return super::commands::complete(token, start..cursor);
        }
        Vec::new()
    }
}

fn is_path_context(prefix: &str, token: &str) -> bool {
    if token.starts_with(['/', '.', '~']) || token.contains('/') {
        return true;
    }
    let command = prefix.split_whitespace().next().unwrap_or_default();
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
        for item in local.complete(&request) {
            assert_eq!(item.replace, 3..5);
        }
    }
}
