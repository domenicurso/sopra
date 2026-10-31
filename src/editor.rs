mod clipboard;
mod commands;
mod completion_state;
mod completions;
mod editing;
mod history;
pub(crate) use history::History;
mod mouse;
mod navigation;
mod pairs;
mod position;
mod session;
pub(crate) mod submission;
#[cfg(test)]
mod tests;
mod text_selection;
#[cfg(test)]
mod text_selection_tests;
mod transcript;
mod view;

use std::{path::PathBuf, time::Instant};

use crate::{
    input::{CursorPosition, TerminalSize},
    palette::TerminalPalette,
    syntax::SyntaxSpan,
};
use sopra_completion::CompletionEngine;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExitReason {
    Accepted,
    Interrupted,
    DelegateTab,
    DelegateEof,
    ChangeDirectory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunResult {
    pub(crate) reason: ExitReason,
    pub(crate) buffer: String,
    pub(crate) cursor: usize,
    pub(crate) highlights: Vec<SyntaxSpan>,
    pub(crate) directory: Option<PathBuf>,
    pub(crate) submitted_spans: Vec<crate::prompt::PromptSpan>,
}

pub(crate) struct EditorConfig {
    pub(crate) buffer: String,
    #[cfg(test)]
    pub(crate) cursor_chars: usize,
    pub(crate) arm_completion: bool,
    pub(crate) suppress_completion: bool,
    pub(crate) completion_enabled: bool,
    pub(crate) syntax_highlighting: bool,
    pub(crate) cursor_animation: bool,
    #[cfg(test)]
    pub(crate) prompt: String,
    #[cfg(test)]
    pub(crate) rprompt: String,
    pub(crate) anchor: CursorPosition,
    pub(crate) size: TerminalSize,
    pub(crate) cwd: PathBuf,
    pub(crate) palette: TerminalPalette,
}

mod session_buffer;
pub(crate) use session_buffer::Draft;

pub(crate) struct EditorState {
    draft: Option<Draft>,
    buffer: String,
    layout: crate::text::layout::Cache,
    history: History,
    cursor: usize,
    prompt: Vec<crate::prompt::PromptSpan>,
    rprompt: Vec<crate::prompt::PromptSpan>,
    transient_prompt: String,
    secondary_prompt: String,
    widgets: crate::prompt::widgets::Widgets,
    anchor: CursorPosition,
    text_selection: text_selection::TextSelection,
    completions: completion_state::State,
    cwd: PathBuf,
    palette: TerminalPalette,
    syntax: Vec<SyntaxSpan>,
    syntax_enabled: bool,
    cursor_animation: bool,
    yank: String,
    animation_started: Instant,
    last_size: TerminalSize,
}

impl EditorState {
    #[cfg(test)]
    pub(crate) fn new(config: EditorConfig) -> Self {
        let cursor = navigation::char_index_to_byte(&config.buffer, config.cursor_chars);
        let widgets = crate::prompt::widgets::Widgets::new(
            config.prompt.clone(),
            config.rprompt.clone(),
            config.cwd.clone(),
        );
        let completion = config
            .completion_enabled
            .then(CompletionEngine::new)
            .flatten();
        let mut state = Self::initialize(config, widgets, cursor, completion);
        state.request_completion();
        state
    }

    pub(crate) fn initialize(
        config: EditorConfig,
        mut widgets: crate::prompt::widgets::Widgets,
        cursor: usize,
        completion: Option<CompletionEngine>,
    ) -> Self {
        let completions = completion_state::State::new(&config, completion);
        let (prompt, rprompt) = widgets.render();
        Self {
            draft: None,
            buffer: config.buffer,
            layout: crate::text::layout::Cache::default(),
            history: History::default(),
            cursor,
            prompt,
            rprompt,
            transient_prompt: "$ ".to_string(),
            secondary_prompt: "> ".to_string(),
            widgets,
            anchor: config.anchor,
            text_selection: text_selection::TextSelection::default(),
            completions,
            cwd: config.cwd,
            palette: config.palette,
            syntax: Vec::new(),
            syntax_enabled: config.syntax_highlighting,
            cursor_animation: config.cursor_animation,
            yank: String::new(),
            animation_started: Instant::now(),
            last_size: config.size,
        }
    }

    pub(crate) fn into_services(
        self,
    ) -> (
        crate::prompt::widgets::Widgets,
        Option<CompletionEngine>,
        History,
    ) {
        (self.widgets, self.completions.engine, self.history)
    }

    pub(super) fn note_activity(&mut self) {
        self.animation_started = Instant::now();
    }
}
