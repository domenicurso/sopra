#[path = "history_navigation.rs"]
mod navigation;
#[path = "history_snapshot.rs"]
mod snapshot;

use std::{io, path::Path};

use sopra_completion::{CompletionItem, CompletionKind, CompletionSource};

use super::EditorState;
use crate::{input::Key, prompt::PromptSpan};

pub(crate) struct History {
    entries: Vec<snapshot::Entry>,
    choice: Option<String>,
    recall: Option<navigation::Recall>,
    enabled: bool,
    format: String,
    metadata_format: String,
    dismissed: Option<String>,
}

impl Default for History {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            choice: None,
            recall: None,
            enabled: true,
            format: "[dim]{text}[/]".to_string(),
            metadata_format: "[dim][italic]{text}[/][/]".to_string(),
            dismissed: None,
        }
    }
}

impl History {
    pub(crate) fn read(path: Option<&Path>) -> io::Result<Self> {
        let mut history = Self::default();
        if let Some(path) = path {
            history.entries = snapshot::read(path)?;
        }
        history.enabled = std::env::var("SOPRA_AUTOSUGGESTIONS").map_or(true, |value| {
            !matches!(value.to_lowercase().as_str(), "0" | "false" | "off" | "no")
        });
        if let Ok(format) = std::env::var("SOPRA_AUTOSUGGESTION_FORMAT") {
            history.format = format;
        }
        if let Ok(format) = std::env::var("SOPRA_AUTOSUGGESTION_METADATA_FORMAT") {
            history.metadata_format = format;
        }
        Ok(history)
    }

    fn matching<'a>(&'a self, buffer: &'a str) -> impl Iterator<Item = &'a snapshot::Entry> {
        self.entries.iter().filter(move |entry| {
            !buffer.trim().is_empty()
                && entry.command.len() > buffer.len()
                && entry.command.starts_with(buffer)
        })
    }

    pub(super) fn completions(&self, buffer: &str, cursor: usize) -> Vec<CompletionItem> {
        if cursor != buffer.len() {
            return Vec::new();
        }
        self.matching(buffer)
            .map(|entry| {
                let mut item = CompletionItem::with_range(
                    entry.command.replace('\n', " ↵ "),
                    entry.command.clone(),
                    Some(snapshot::description(entry)),
                    CompletionKind::History,
                    0..buffer.len(),
                    CompletionSource::ShellContext,
                );
                item.group = Some("History".to_string());
                item.match_indices = buffer.char_indices().map(|(byte, _)| byte).collect();
                item
            })
            .collect()
    }

    fn chosen<'a>(&'a self, buffer: &'a str) -> Option<&'a str> {
        let entries = self.matching(buffer).collect::<Vec<_>>();
        self.choice
            .as_deref()
            .filter(|choice| entries.iter().any(|entry| entry.command == *choice))
            .or_else(|| entries.first().map(|entry| entry.command.as_str()))
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
            || self.text_selection.range().is_some()
            || self.history.dismissed.as_deref() == Some(&self.buffer)
        {
            return None;
        }
        let command =
            if let Some(item) = self.selected.and_then(|index| self.suggestions.get(index)) {
                (item.kind == CompletionKind::History).then_some(item.insert.as_str())?
            } else {
                self.history.chosen(&self.buffer)?
            };
        command.strip_prefix(&self.buffer)
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

    pub(crate) fn autosuggestion_metadata(&self) -> Vec<PromptSpan> {
        let Some(suffix) = self.history_suffix() else {
            return Vec::new();
        };
        let entry = self
            .history
            .entries
            .iter()
            .find(|entry| entry.command.strip_prefix(&self.buffer) == Some(suffix));
        entry.map_or_else(Vec::new, |entry| {
            crate::prompt::parse(
                &self
                    .history
                    .metadata_format
                    .replace("{text}", &snapshot::description(entry)),
            )
        })
    }

    pub(super) fn accept_history(&mut self, key: Key) -> bool {
        if !matches!(key, Key::Right | Key::End | Key::WordRight | Key::Tab) {
            return false;
        }
        let Some(suffix) = self.history_suffix() else {
            return false;
        };
        let chosen = format!("{}{}", self.buffer, suffix);
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
        self.history.choice = Some(chosen);
        self.buffer.push_str(&accepted);
        self.cursor = self.buffer.len();
        self.reset_selection();
        true
    }
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
