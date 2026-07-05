use keel_core::HistoryEntry;

#[derive(Debug, Clone, Default)]
pub struct HistoryIndex {
    entries: Vec<HistoryEntry>,
}

impl HistoryIndex {
    pub fn from_zsh_history(input: &str) -> Self {
        let mut entries = Vec::new();

        for line in input.lines() {
            if let Some(rest) = line.strip_prefix(": ") {
                let mut parts = rest.splitn(2, ';');
                let metadata = parts.next().unwrap_or_default();
                let command = parts.next().unwrap_or_default().to_string();
                let timestamp = metadata
                    .split(':')
                    .next()
                    .and_then(|value| value.trim().parse::<i64>().ok());

                entries.push(HistoryEntry {
                    command,
                    cwd: None,
                    exit_status: None,
                    duration_ms: None,
                    timestamp,
                });
            } else if !line.trim().is_empty() {
                entries.push(HistoryEntry {
                    command: line.to_string(),
                    cwd: None,
                    exit_status: None,
                    duration_ms: None,
                    timestamp: None,
                });
            }
        }

        Self { entries }
    }

    pub fn push_session_entry(&mut self, command: String) {
        self.entries.push(HistoryEntry {
            command,
            cwd: None,
            exit_status: None,
            duration_ms: None,
            timestamp: None,
        });
    }

    pub fn suggest(&self, prefix: &str) -> Option<String> {
        self.entries
            .iter()
            .rev()
            .find(|entry| entry.command.starts_with(prefix) && entry.command != prefix)
            .map(|entry| entry.command.clone())
    }

    pub fn search(&self, needle: &str) -> Vec<HistoryEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.command.contains(needle))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::HistoryIndex;

    #[test]
    fn parses_zsh_extended_history_and_suggests() {
        let index = HistoryIndex::from_zsh_history(": 1700000000:0;git status\npwd\n");
        assert_eq!(index.suggest("git"), Some("git status".to_string()));
        assert_eq!(index.search("pw").len(), 1);
    }
}
