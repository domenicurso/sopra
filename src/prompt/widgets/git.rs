use std::{path::Path, process::Command};

use super::Widget;

pub(super) struct Git(String);

impl Git {
    pub(super) fn new(cwd: &Path, enabled: bool) -> Self {
        if !enabled {
            return Self(String::new());
        }
        let branch = git(cwd, &["symbolic-ref", "--quiet", "--short", "HEAD"])
            .or_else(|| git(cwd, &["rev-parse", "--short", "HEAD"]))
            .unwrap_or_default();
        Self(
            branch
                .chars()
                .filter(|character| !character.is_control())
                .collect(),
        )
    }
}

impl Widget for Git {
    fn token(&self) -> &'static str {
        "{git}"
    }
    fn render(&self) -> String {
        self.0.clone()
    }
}

fn git(cwd: &Path, arguments: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(arguments)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_string(),
    )
}
