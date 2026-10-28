use std::{collections::HashSet, io, path::Path};

use sopra_completion::{CompletionItem, CompletionKind, CompletionSource};

use super::EditorState;
use crate::{input::Key, prompt::PromptSpan};

pub(crate) struct History {
    entries: Vec<String>,
    enabled: bool,
    format: String,
    dismissed: Option<String>,
}

impl Default for History {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            enabled: true,
            format: "[dim]{text}[/]".to_string(),
            dismissed: None,
        }
    }
}

impl History {
    pub(crate) fn read(path: Option<&Path>) -> io::Result<Self> {
        let mut history = Self::default();
        if let Some(path) = path {
            let bytes = std::fs::read(path)?;
            let mut seen = HashSet::new();
            history.entries = bytes
                .split(|byte| *byte == 0)
                .filter_map(|record| {
                    // The editor accepts UTF-8 and printable commands; other records cannot
                    // be displayed or inserted faithfully and must not become suggestions.
                    let text = std::str::from_utf8(record).ok()?;
                    (!text.trim().is_empty()
                        && !text.chars().any(|c| c.is_control() && c != '\n')
                        && seen.insert(text.to_string()))
                    .then(|| text.to_string())
                })
                .collect();
        }
        history.enabled = std::env::var("SOPRA_AUTOSUGGESTIONS").map_or(true, |value| {
            !matches!(value.to_lowercase().as_str(), "0" | "false" | "off" | "no")
        });
        if let Ok(format) = std::env::var("SOPRA_AUTOSUGGESTION_FORMAT") {
            history.format = format;
        }
        Ok(history)
    }

    fn matching<'a>(&'a self, buffer: &'a str) -> impl Iterator<Item = &'a String> {
        self.entries.iter().filter(move |entry| {
            !buffer.trim().is_empty() && entry.len() > buffer.len() && entry.starts_with(buffer)
        })
    }

    pub(super) fn completions(&self, buffer: &str, cursor: usize) -> Vec<CompletionItem> {
        if cursor != buffer.len() {
            return Vec::new();
        }
        self.matching(buffer)
            .map(|entry| {
                let mut item = CompletionItem::with_range(
                    entry.replace('\n', " ↵ "),
                    entry.clone(),
                    Some("History".to_string()),
                    CompletionKind::History,
                    0..buffer.len(),
                    CompletionSource::ShellContext,
                );
                item.group = Some("History".to_string());
                item.match_indices = (0..buffer.chars().count()).collect();
                item
            })
            .collect()
    }

    pub(super) fn dismiss(&mut self, buffer: &str) {
        self.dismissed = Some(buffer.to_string());
    }
}

impl EditorState {
    pub(crate) fn set_history(&mut self, history: History) {
        self.history = history;
        self.refresh_suggestions();
    }

    pub(super) fn history_suffix(&self) -> Option<&str> {
        if !self.history.enabled
            || self.cursor != self.buffer.len()
            || self.widgets.active()
            || self.selected.is_some()
            || self.text_selection.range().is_some()
            || self.history.dismissed.as_deref() == Some(&self.buffer)
        {
            return None;
        }
        self.history
            .matching(&self.buffer)
            .next()
            .map(|entry| &entry[self.buffer.len()..])
    }

    pub(crate) fn autosuggestion_spans(&self) -> Vec<PromptSpan> {
        self.history_suffix().map_or_else(Vec::new, |suffix| {
            crate::prompt::parse(
                &self
                    .history
                    .format
                    .replace("{text}", &suffix.replace('[', "[[")),
            )
        })
    }

    pub(super) fn accept_history(&mut self, key: Key) -> bool {
        if !matches!(key, Key::Right | Key::End | Key::WordRight) {
            return false;
        }
        let Some(suffix) = self.history_suffix() else {
            return false;
        };
        let count = if key == Key::WordRight {
            let start = suffix.len() - suffix.trim_start().len();
            start
                + suffix[start..]
                    .find(char::is_whitespace)
                    .unwrap_or(suffix.len() - start)
        } else {
            suffix.len()
        };
        let accepted = suffix[..count].to_string();
        self.buffer.push_str(&accepted);
        self.cursor = self.buffer.len();
        self.reset_selection();
        true
    }
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
