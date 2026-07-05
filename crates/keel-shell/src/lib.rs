use std::{env, path::PathBuf};

use keel_core::ShellSnapshot;

#[derive(Debug, Default)]
pub struct ShellContextCollector;

impl ShellContextCollector {
    pub fn capture() -> ShellSnapshot {
        let cwd = env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .display()
            .to_string();

        let mut environment = env::vars().collect::<Vec<_>>();
        environment.sort_by(|left, right| left.0.cmp(&right.0));

        ShellSnapshot {
            cwd,
            last_status: env::var("KEEL_LAST_STATUS")
                .ok()
                .and_then(|value| value.parse::<i32>().ok())
                .unwrap_or_default(),
            last_duration_ms: env::var("KEEL_LAST_DURATION_MS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok()),
            aliases: Vec::new(),
            functions: Vec::new(),
            environment,
        }
    }
}
