use std::{path::Path, process::Command};

use super::Widget;

pub(super) struct GitStatus(String);

impl GitStatus {
    pub(super) fn new(cwd: &Path, enabled: bool) -> Self {
        let text = if enabled {
            Command::new("git")
                .current_dir(cwd)
                .args([
                    "--no-optional-locks",
                    "status",
                    "--porcelain=v2",
                    "--branch",
                ])
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|output| summarize(&String::from_utf8_lossy(&output.stdout)))
                .unwrap_or_default()
        } else {
            String::new()
        };
        Self(text)
    }
}

impl Widget for GitStatus {
    fn token(&self) -> &'static str {
        "{git_status}"
    }
    fn render(&self) -> String {
        self.0.clone()
    }
}

fn summarize(output: &str) -> String {
    let (mut changed, mut untracked, mut ahead, mut behind) = (0, 0, 0, 0);
    for line in output.lines() {
        match line.as_bytes().first() {
            Some(b'?') => untracked += 1,
            Some(b'1' | b'2' | b'u') => changed += 1,
            Some(b'#') if line.starts_with("# branch.ab ") => {
                let mut parts = line[12..].split_whitespace();
                ahead = parts
                    .next()
                    .and_then(|part| part.strip_prefix('+'))
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0);
                behind = parts
                    .next()
                    .and_then(|part| part.strip_prefix('-'))
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0);
            }
            _ => {}
        }
    }
    let mut parts = Vec::new();
    if changed > 0 {
        parts.push(format!("{changed}!"));
    }
    if untracked > 0 {
        parts.push(format!("{untracked}?"));
    }
    if ahead > 0 {
        parts.push(format!("{ahead}↑"));
    }
    if behind > 0 {
        parts.push(format!("{behind}↓"));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("[{}]", parts.join("|"))
    }
}

#[cfg(test)]
mod tests {
    use super::summarize;
    #[test]
    fn summarizes_changes_and_upstream_counts() {
        assert_eq!(
            summarize("# branch.ab +3 -1\n1 M. file\n? new\n"),
            "[1!|1?|3↑|1↓]"
        );
        assert_eq!(summarize("# branch.ab +0 -0\n"), "");
    }
}
