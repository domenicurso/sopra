use sopra::{Context, Emphasis, Text, Widget};
use std::process::Command;

pub(crate) struct Snapshot {
    branch: String,
    counts: [usize; 4],
}
struct Git;
impl Widget for Git {
    fn render(&mut self, context: &Context) -> Text {
        let Some(snapshot) = snapshot(context) else {
            return Text::new();
        };
        let mut text = Text::new()
            .part(Emphasis::Dim, context.option("label", "on "))
            .push(&snapshot.branch);
        let status = snapshot.status(context);
        if !status.parts.is_empty() {
            text = text.push(" ");
            text.parts.extend(status.parts);
        }
        text
    }
}

pub(crate) fn snapshot(context: &Context) -> Option<Snapshot> {
    let output = Command::new("git")
        .env_clear()
        .envs(&context.environment)
        .current_dir(&context.cwd)
        .args([
            "--no-optional-locks",
            "status",
            "--porcelain=v2",
            "--branch",
        ])
        .output()
        .ok()
        .filter(|output| output.status.success())?;
    Some(parse(&String::from_utf8_lossy(&output.stdout)))
}

fn parse(output: &str) -> Snapshot {
    let mut result = Snapshot {
        branch: String::new(),
        counts: [0; 4],
    };
    let mut commit = "";
    for line in output.lines() {
        match line.as_bytes().first() {
            Some(b'?') => result.counts[1] += 1,
            Some(b'1' | b'2' | b'u') => result.counts[0] += 1,
            Some(b'#') if line.starts_with("# branch.oid ") => commit = &line[13..],
            Some(b'#') if line.starts_with("# branch.head ") => result.branch = line[14..].into(),
            Some(b'#') if line.starts_with("# branch.ab ") => {
                let values = line[12..]
                    .split_whitespace()
                    .filter_map(|v| v.get(1..)?.parse().ok())
                    .collect::<Vec<_>>();
                if values.len() == 2 {
                    result.counts[2..].copy_from_slice(&values);
                }
            }
            _ => {}
        }
    }
    if result.branch == "(detached)" {
        result.branch = commit.chars().take(7).collect();
    }
    result
}

impl Snapshot {
    pub(crate) fn status(&self, context: &Context) -> Text {
        let symbols = [
            ("changed", "!"),
            ("untracked", "?"),
            ("ahead", "↑"),
            ("behind", "↓"),
        ];
        let items = self
            .counts
            .iter()
            .zip(symbols)
            .filter(|(count, _)| **count > 0)
            .map(|(count, (key, default))| format!("{count}{}", context.option(key, default)))
            .collect::<Vec<_>>();
        if items.is_empty() {
            return Text::new();
        }
        let mut text = Text::new().part(Emphasis::Dim, context.option("open", "["));
        for (index, item) in items.into_iter().enumerate() {
            if index > 0 {
                text = text.part(Emphasis::Dim, context.option("separator", "|"));
            }
            text = text.push(item);
        }
        text.part(Emphasis::Dim, context.option("close", "]"))
    }
}

pub(crate) fn create() -> Box<dyn Widget + Send> {
    Box::new(Git)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn git_content_options_change_symbols_without_styling() {
        let snapshot = parse("# branch.head main\n# branch.ab +3 -1\n1 M. file\n? new\n");
        let mut context = Context::default();
        context.options.insert("changed".into(), "*".into());
        let text = snapshot.status(&context);
        assert_eq!(
            text.parts
                .iter()
                .map(|p| p.text.as_str())
                .collect::<String>(),
            "[1*|1?|3↑|1↓]"
        );
        assert_eq!(snapshot.branch, "main");
        assert_eq!(
            parse("# branch.oid abcdef123456\n# branch.head (detached)\n").branch,
            "abcdef1"
        );
        assert!(
            parse("# branch.ab +0 -0\n")
                .status(&context)
                .parts
                .is_empty()
        );
    }
}
