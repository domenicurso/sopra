mod clipboard;
mod commands;
mod completions;
mod editing;
mod mouse;
mod navigation;
mod pairs;
mod session;
#[cfg(test)]
mod tests;
mod text_selection;
#[cfg(test)]
mod text_selection_tests;
mod view;

use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::{
    input::{CursorPosition, TerminalSize},
    palette::TerminalPalette,
    syntax::SyntaxSpan,
};
use sopra_completion::{CompletionEngine, CompletionItem};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExitReason {
    Accepted,
    Interrupted,
    DelegateUp,
    DelegateDown,
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
}

pub(crate) struct EditorConfig {
    pub(crate) buffer: String,
    pub(crate) cursor_chars: usize,
    pub(crate) arm_completion: bool,
    pub(crate) suppress_completion: bool,
    pub(crate) completion_enabled: bool,
    pub(crate) syntax_highlighting: bool,
    pub(crate) cursor_animation: bool,
    pub(crate) prompt: String,
    pub(crate) rprompt: String,
    pub(crate) anchor: CursorPosition,
    pub(crate) size: TerminalSize,
    pub(crate) cwd: PathBuf,
    pub(crate) palette: TerminalPalette,
}

pub(crate) struct EditorState {
    buffer: String,
    cursor: usize,
    prompt: String,
    rprompt: String,
    transient_prompt: String,
    widgets: crate::prompt::widgets::Widgets,
    anchor: CursorPosition,
    selected: Option<usize>,
    text_selection: text_selection::TextSelection,
    completion_armed: bool,
    completion_suspended: bool,
    suggestion_scroll: usize,
    overlay_visible: bool,
    completion_source: Vec<CompletionItem>,
    suggestions: Vec<CompletionItem>,
    completion_key: String,
    latest_local_generation: u64,
    completion: Option<CompletionEngine>,
    completion_elapsed: Duration,
    completion_latency_generation: u64,
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
    pub(crate) fn new(config: EditorConfig) -> Self {
        let cursor = navigation::char_index_to_byte(&config.buffer, config.cursor_chars);
        let completion_active = config.completion_enabled && !config.suppress_completion;
        let widgets = crate::prompt::widgets::Widgets::new(
            config.prompt.clone(),
            config.rprompt.clone(),
            config.cwd.clone(),
        );
        let (prompt, rprompt) = widgets.render();
        let mut state = Self {
            buffer: config.buffer,
            cursor,
            prompt,
            rprompt,
            transient_prompt: "$ ".to_string(),
            widgets,
            anchor: config.anchor,
            text_selection: text_selection::TextSelection::default(),
            selected: (config.arm_completion && completion_active).then_some(0),
            completion_armed: config.arm_completion && completion_active,
            completion_suspended: config.suppress_completion || !config.completion_enabled,
            suggestion_scroll: 0,
            overlay_visible: config.completion_enabled && !config.suppress_completion,
            completion_source: Vec::new(),
            suggestions: Vec::new(),
            completion_key: String::new(),
            latest_local_generation: 0,
            completion: config
                .completion_enabled
                .then(CompletionEngine::new)
                .flatten(),
            completion_elapsed: Duration::ZERO,
            completion_latency_generation: 0,
            cwd: config.cwd,
            palette: config.palette,
            syntax: Vec::new(),
            syntax_enabled: config.syntax_highlighting,
            cursor_animation: config.cursor_animation,
            yank: String::new(),
            animation_started: Instant::now(),
            last_size: config.size,
        };
        state.request_completion();
        state
    }

    pub(super) fn note_activity(&mut self) {
        self.animation_started = Instant::now();
    }
}
