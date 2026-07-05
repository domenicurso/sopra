use std::{env, fs, path::Path};

use keel_core::{
    CompletionGroup, CompletionItem, CompletionKind, CompletionRequest, CompletionResponse,
};
use keel_shell::ShellContextCollector;

#[derive(Debug, Default)]
pub struct CompletionRegistry;

impl CompletionRegistry {
    pub fn complete(&self, request: &CompletionRequest) -> CompletionResponse {
        let prefix = request.buffer[..request.cursor.min(request.buffer.len())]
            .split_whitespace()
            .last()
            .unwrap_or_default()
            .to_string();
        let replacement_start = request.cursor.saturating_sub(prefix.chars().count());
        let replacement_end = request.cursor;

        let mut groups = Vec::new();

        let commands = self.command_candidates(&prefix, replacement_start, replacement_end);
        if !commands.is_empty() {
            groups.push(CompletionGroup {
                name: "commands".to_string(),
                items: commands,
            });
        }

        let files = self.file_candidates(&request.cwd, &prefix, replacement_start, replacement_end);
        if !files.is_empty() {
            groups.push(CompletionGroup {
                name: "files".to_string(),
                items: files,
            });
        }

        let env_vars = self.environment_candidates(&prefix, replacement_start, replacement_end);
        if !env_vars.is_empty() {
            groups.push(CompletionGroup {
                name: "environment".to_string(),
                items: env_vars,
            });
        }

        CompletionResponse { groups }
    }

    fn command_candidates(
        &self,
        prefix: &str,
        replacement_start: usize,
        replacement_end: usize,
    ) -> Vec<CompletionItem> {
        let mut commands = vec!["cd", "echo", "git", "grep", "ls", "pwd", "zed"]
            .into_iter()
            .filter(|command| command.starts_with(prefix))
            .map(|command| CompletionItem {
                label: command.to_string(),
                insert_text: command.to_string(),
                description: Some("builtin/native command source".to_string()),
                kind: CompletionKind::Command,
                replacement_start,
                replacement_end,
                group: "commands".to_string(),
            })
            .collect::<Vec<_>>();
        commands.sort_by(|left, right| left.label.cmp(&right.label));
        commands
    }

    fn file_candidates(
        &self,
        cwd: &str,
        prefix: &str,
        replacement_start: usize,
        replacement_end: usize,
    ) -> Vec<CompletionItem> {
        let base = Path::new(cwd);
        let Ok(read_dir) = fs::read_dir(base) else {
            return Vec::new();
        };

        let mut items = read_dir
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let path = entry.path();
                let file_name = entry.file_name().to_string_lossy().to_string();
                if !file_name.starts_with(prefix) {
                    return None;
                }

                let kind = if path.is_dir() {
                    CompletionKind::Directory
                } else {
                    CompletionKind::File
                };

                Some(CompletionItem {
                    label: file_name.clone(),
                    insert_text: file_name,
                    description: Some(path.display().to_string()),
                    kind,
                    replacement_start,
                    replacement_end,
                    group: "files".to_string(),
                })
            })
            .collect::<Vec<_>>();
        items.sort_by(|left, right| left.label.cmp(&right.label));
        items
    }

    fn environment_candidates(
        &self,
        prefix: &str,
        replacement_start: usize,
        replacement_end: usize,
    ) -> Vec<CompletionItem> {
        let _snapshot = ShellContextCollector::capture();
        let mut items = env::vars()
            .filter(|(name, _)| name.starts_with(prefix))
            .map(|(name, value)| CompletionItem {
                label: name.clone(),
                insert_text: name,
                description: Some(value),
                kind: CompletionKind::Environment,
                replacement_start,
                replacement_end,
                group: "environment".to_string(),
            })
            .collect::<Vec<_>>();
        items.sort_by(|left, right| left.label.cmp(&right.label));
        items
    }
}

#[cfg(test)]
mod tests {
    use super::CompletionRegistry;
    use keel_core::CompletionRequest;

    #[test]
    fn returns_grouped_native_completion_items() {
        let registry = CompletionRegistry;
        let response = registry.complete(&CompletionRequest {
            buffer: "gr".to_string(),
            cursor: 2,
            cwd: ".".to_string(),
        });

        assert!(response.groups.iter().any(|group| group.name == "commands"));
    }
}
