#[path = "history_affinity.rs"]
mod affinity;
#[path = "history_navigation.rs"]
mod navigation;
#[path = "history_search.rs"]
mod search;
#[path = "history_snapshot.rs"]
mod snapshot;

use std::{io, path::Path};

use super::EditorState;
use crate::{actions::Action, prompt::PromptSpan};

pub(crate) struct History {
    entries: Vec<snapshot::Entry>,
    search: Option<search::Search>,
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
            search: None,
            choice: None,
            recall: None,
            enabled: true,
            format: "[dim]$text[/dim]".to_string(),
            metadata_format: "[dim italic]$text[/dim italic]".to_string(),
            dismissed: None,
        }
    }
}

impl History {
    pub(crate) fn refresh(
        &mut self,
        path: Option<&Path>,
        config: &crate::config::Config,
    ) -> io::Result<()> {
        self.entries = path.map(snapshot::read).transpose()?.unwrap_or_default();
        self.search = None;
        self.choice = None;
        self.recall = None;
        self.dismissed = None;
        self.enabled = config.autosuggestions;
        self.format = config.autosuggestion_format.clone();
        self.metadata_format = config.autosuggestion_metadata_format.clone();
        Ok(())
    }

    fn matching<'a>(&'a self, buffer: &'a str) -> impl Iterator<Item = &'a snapshot::Entry> {
        self.entries.iter().filter(move |entry| {
            !buffer.trim().is_empty()
                && entry.command.len() > buffer.len()
                && entry.command.starts_with(buffer)
        })
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
            || self.searching_history()
            || self.text_selection.range().is_some()
            || self.history.dismissed.as_deref() == Some(&self.buffer)
        {
            return None;
        }
        if self.overlay_visible() && self.completions.focus.selected().is_some() {
            return None;
        }
        let command = self.history.chosen(&self.buffer)?;
        command.strip_prefix(&self.buffer)
    }

    pub(crate) fn autosuggestion_spans(&self) -> Vec<PromptSpan> {
        self.history_suffix().map_or_else(Vec::new, |suffix| {
            render_history(&self.history.format, suffix)
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
            render_history(&self.history.metadata_format, &snapshot::description(entry))
        })
    }

    pub(super) fn accept_history(&mut self, key: Action) -> bool {
        if !matches!(key, Action::Right | Action::End | Action::WordRight) {
            return false;
        }
        let Some(suffix) = self.history_suffix() else {
            return false;
        };
        let chosen = format!("{}{}", self.buffer, suffix);
        let count = if key == Action::WordRight {
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

fn render_history(source: &str, value: &str) -> Vec<PromptSpan> {
    crate::prompt::template::Template::compile(source)
        .map(|template| {
            template.render(&std::collections::BTreeMap::from([(
                "text".into(),
                value.into(),
            )]))
        })
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
