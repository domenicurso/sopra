use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{
    CompletionItem, CompletionKind, Request,
    ranking::{byte_offset, token_range},
};

const MAX_LOCAL_ITEMS: usize = 512;

pub(super) fn complete(request: &Request) -> Vec<CompletionItem> {
    let cursor = byte_offset(&request.line, request.cursor);
    let (start, _) = token_range(&request.line, cursor);
    let token = &request.line[start..cursor];
    if is_path_context(&request.line[..start], token) {
        return complete_path(request, token);
    }
    if crate::syntax::command_position(&request.line, cursor) {
        return super::commands::complete(token);
    }
    Vec::new()
}

fn complete_path(request: &Request, token: &str) -> Vec<CompletionItem> {
    let (option_prefix, _path, path_parent, leaf) = path_parts(token);
    let directory = resolve_path(path_parent, &request.cwd);
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut names = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let hidden = name.starts_with('.') && !leaf.starts_with('.');
            (!hidden && name != "." && name != "..").then_some((name, entry.path()))
        })
        .filter(|(name, _)| {
            !name.is_empty() && (leaf.is_empty() || name.chars().count() >= leaf.chars().count())
        })
        .collect::<Vec<_>>();
    names.sort_by(|left, right| {
        left.0
            .to_ascii_lowercase()
            .cmp(&right.0.to_ascii_lowercase())
            .then_with(|| left.0.cmp(&right.0))
    });
    names
        .into_iter()
        .take(MAX_LOCAL_ITEMS)
        .map(|(name, entry_path)| {
            let directory = entry_path.is_dir();
            let suffix = if directory { "/" } else { "" };
            let label = format!("{name}{suffix}");
            let replacement = format!("{option_prefix}{path_parent}{name}{suffix}");
            CompletionItem::new(
                label,
                "",
                replacement,
                if directory {
                    CompletionKind::Directory
                } else {
                    CompletionKind::File
                },
            )
        })
        .collect()
}

fn is_path_context(prefix: &str, token: &str) -> bool {
    let path_parent = path_parts(token).2;
    if token.starts_with(['/', '.', '~']) || path_parent.contains('/') {
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

fn path_parts(token: &str) -> (&str, &str, &str, &str) {
    let value_start = token
        .find('=')
        .filter(|_| token.starts_with('-'))
        .map_or(0, |index| index + 1);
    let option_prefix = &token[..value_start];
    let path = &token[value_start..];
    let slash = path.rfind('/');
    let (path_parent, leaf) =
        slash.map_or(("", path), |index| (&path[..=index], &path[index + 1..]));
    (option_prefix, path, path_parent, leaf)
}

fn resolve_path(parent: &str, cwd: &Path) -> PathBuf {
    if parent.is_empty() {
        return cwd.to_path_buf();
    }
    if let Some(rest) = parent.strip_prefix('~')
        && let Ok(home) = std::env::var("HOME")
    {
        return PathBuf::from(home).join(rest.trim_start_matches('/'));
    }
    if Path::new(parent).is_absolute() {
        PathBuf::from(parent)
    } else {
        cwd.join(parent)
    }
}

#[cfg(test)]
mod tests {
    use super::path_parts;

    #[test]
    fn path_parts_preserve_option_prefix_and_parent() {
        assert_eq!(
            path_parts("--file=./src/ed"),
            ("--file=", "./src/ed", "./src/", "ed")
        );
        assert_eq!(path_parts("/usr/sh"), ("", "/usr/sh", "/usr/", "sh"));
        assert_eq!(path_parts("command"), ("", "command", "", "command"));
    }
}
