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
    fn markup(&self) -> bool {
        true
    }
    fn render(&self) -> String {
        if self.0.is_empty() {
            return String::new();
        }
        let parts = self.0[1..self.0.len() - 1].split('|');
        let separator = "[default][dim]|[/][/]";
        format!(
            "[default][dim][[[/][/]{}[default][dim]][/][/]",
            parts
                .map(|part| format!("[bold]{part}[/]"))
                .collect::<Vec<_>>()
                .join(separator)
        )
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
    fn status_keeps_values_bold_and_punctuation_dim() {
        use super::Widget;
        use ratatui::style::{Color, Modifier};
        let text = super::GitStatus("[13!|2?|6↑]".to_string()).render();
        let spans = crate::prompt::parse(&format!("[magenta]{text}[/]"));
        assert_eq!(
            spans
                .iter()
                .map(|span| span.text.as_str())
                .collect::<String>(),
            "[13!|2?|6↑]"
        );
        for span in spans {
            if span.text.chars().all(|ch| "[]|".contains(ch)) {
                assert_eq!(span.style.fg, Some(Color::Reset));
                assert!(span.style.add_modifier.contains(Modifier::DIM));
            } else {
                assert_eq!(span.style.fg, Some(Color::Magenta));
                assert!(span.style.add_modifier.contains(Modifier::BOLD));
            }
        }
    }
    #[test]
    fn summarizes_changes_and_upstream_counts() {
        assert_eq!(
            summarize("# branch.ab +3 -1\n1 M. file\n? new\n"),
            "[1!|1?|3↑|1↓]"
        );
        assert_eq!(summarize("# branch.ab +0 -0\n"), "");
    }
}
