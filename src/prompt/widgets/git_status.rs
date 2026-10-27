use std::{path::Path, process::Command};

use super::Widget;

pub(super) struct GitStatus {
    text: String,
    format: String,
    separator: String,
    item_format: String,
}

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
        Self {
            text,
            format: std::env::var("SOPRA_GIT_STATUS_FORMAT")
                .unwrap_or_else(|_| "[[{items}]".into()),
            separator: std::env::var("SOPRA_GIT_STATUS_SEPARATOR").unwrap_or_else(|_| "|".into()),
            item_format: std::env::var("SOPRA_GIT_STATUS_ITEM_FORMAT")
                .unwrap_or_else(|_| "{item}".into()),
        }
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
        if self.text.is_empty() {
            return String::new();
        }
        let parts = self.text[1..self.text.len() - 1].split('|');
        let items = parts
            .map(|part| self.item_format.replace("{item}", part))
            .collect::<Vec<_>>()
            .join(&self.separator);
        self.format.replace("{items}", &items)
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
    fn status_formats_values_and_separators_from_configuration() {
        use super::Widget;
        use ratatui::style::{Color, Modifier};
        let status = super::GitStatus {
            text: "[13!|2?|6↑]".into(),
            format: "[dim][[[/]{items}[dim]][/]".into(),
            item_format: "[bold][magenta]{item}[/][/]".into(),
            separator: "[dim]|[/]".into(),
        };
        let spans = crate::prompt::parse(&status.render());
        assert_eq!(
            spans
                .iter()
                .map(|span| span.text.as_str())
                .collect::<String>(),
            "[13!|2?|6↑]"
        );
        assert!(
            spans.iter().filter(|span| span.text == "|").all(|span| span
                .style
                .add_modifier
                .contains(Modifier::DIM)
                && !span.style.add_modifier.contains(Modifier::BOLD))
        );
        assert!(
            spans
                .iter()
                .filter(|span| span.text.contains('!'))
                .all(|span| span.style.fg == Some(Color::Magenta)
                    && span.style.add_modifier.contains(Modifier::BOLD))
        );
        let plain = super::GitStatus {
            format: "{items}".into(),
            item_format: "{item}".into(),
            separator: " / ".into(),
            ..status
        };
        assert_eq!(plain.render(), "13! / 2? / 6↑");
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
