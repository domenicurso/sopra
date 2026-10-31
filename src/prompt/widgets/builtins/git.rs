use std::{path::Path, process::Command};

use super::Widget;

pub(super) struct Git(String);

impl Git {
    pub(super) fn new(cwd: &Path) -> Self {
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
    fn token(&self) -> &str {
        "{git}"
    }
    fn render(&self) -> String {
        self.0.clone()
    }
}

fn git(cwd: &Path, arguments: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .env_clear().envs(sopra_shell_context::vars_os())
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

pub(super) fn create(context: &super::WidgetContext<'_>) -> Box<dyn Widget> {
    Box::new(Git::new(context.cwd))
}
