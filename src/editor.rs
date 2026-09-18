mod commands;
mod completions;
mod editing;
mod navigation;
#[cfg(test)]
mod tests;
mod view;

use std::{
    io,
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::{
    completion::{CompletionEngine, CompletionItem},
    input::{CursorPosition, Terminal, TerminalSize},
    palette::TerminalPalette,
    render::Renderer,
    scene::Scene,
    syntax::SyntaxSpan,
};
const FRAME_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExitReason {
    Accepted,
    Interrupted,
    DelegateUp,
    DelegateDown,
    DelegateTab,
    DelegateEof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunResult {
    pub(crate) reason: ExitReason,
    pub(crate) buffer: String,
    pub(crate) cursor: usize,
}

pub(crate) struct EditorConfig {
    pub(crate) buffer: String,
    pub(crate) cursor_chars: usize,
    pub(crate) prompt: String,
    pub(crate) anchor: CursorPosition,
    pub(crate) size: TerminalSize,
    pub(crate) cwd: PathBuf,
    pub(crate) palette: TerminalPalette,
}

pub(crate) struct EditorState {
    buffer: String,
    cursor: usize,
    prompt: String,
    anchor: CursorPosition,
    selected: usize,
    suggestion_scroll: usize,
    overlay_visible: bool,
    completion_source: Vec<CompletionItem>,
    zshrs_source: Vec<CompletionItem>,
    suggestions: Vec<CompletionItem>,
    completion_key: String,
    latest_local_generation: u64,
    latest_zshrs_generation: u64,
    completion: Option<CompletionEngine>,
    completion_elapsed: Duration,
    completion_latency_generation: u64,
    cwd: PathBuf,
    palette: TerminalPalette,
    syntax: Vec<SyntaxSpan>,
    yank: String,
    animation_started: Instant,
    last_size: TerminalSize,
}

impl EditorState {
    pub(crate) fn new(config: EditorConfig) -> Self {
        let cursor = navigation::char_index_to_byte(&config.buffer, config.cursor_chars);
        let mut state = Self {
            buffer: config.buffer,
            cursor,
            prompt: config.prompt,
            anchor: config.anchor,
            selected: 0,
            suggestion_scroll: 0,
            overlay_visible: true,
            completion_source: Vec::new(),
            zshrs_source: Vec::new(),
            suggestions: Vec::new(),
            completion_key: String::new(),
            latest_local_generation: 0,
            latest_zshrs_generation: 0,
            completion: CompletionEngine::new(),
            completion_elapsed: Duration::ZERO,
            completion_latency_generation: 0,
            cwd: config.cwd,
            palette: config.palette,
            syntax: Vec::new(),
            yank: String::new(),
            animation_started: Instant::now(),
            last_size: config.size,
        };
        state.request_completion();
        state
    }

    pub(crate) fn run(&mut self, terminal: &mut Terminal) -> io::Result<RunResult> {
        let mut renderer = Renderer::default();
        let mut next_frame = Instant::now();
        loop {
            self.poll_completion();
            let now = Instant::now();
            if now >= next_frame {
                self.paint_editor(now, terminal, &mut renderer)?;
                next_frame = now + FRAME_INTERVAL;
            }

            let wait = next_frame.saturating_duration_since(Instant::now());
            if terminal.poll(wait)? {
                if let Some(result) = self.handle_key(terminal.read_key()?) {
                    if result.reason == ExitReason::Interrupted {
                        self.finish(&mut renderer, terminal, result.reason)?;
                        self.start_fresh_line(terminal)?;
                        renderer = Renderer::default();
                        next_frame = Instant::now();
                        continue;
                    }
                    self.finish(&mut renderer, terminal, result.reason)?;
                    return Ok(result);
                }
                next_frame = Instant::now();
            }
        }
    }

    fn paint_editor(
        &mut self,
        now: Instant,
        terminal: &mut Terminal,
        renderer: &mut Renderer,
    ) -> io::Result<()> {
        let size = self.refresh_size(terminal);
        let frame = Scene::for_editor(self, size, now).render();
        renderer.render(frame, terminal)?;
        Ok(())
    }

    fn refresh_size(&mut self, terminal: &Terminal) -> TerminalSize {
        let size = terminal.size().unwrap_or(self.last_size);
        self.last_size = size;
        size
    }

    pub(super) fn note_activity(&mut self) {
        self.animation_started = Instant::now();
    }

    fn start_fresh_line(&mut self, terminal: &mut Terminal) -> io::Result<()> {
        terminal.write_all(b"\r\n")?;
        terminal.flush()?;
        self.buffer.clear();
        self.cursor = 0;
        self.selected = 0;
        self.suggestion_scroll = 0;
        self.completion_source.clear();
        self.zshrs_source.clear();
        self.suggestions.clear();
        self.completion_key.clear();
        self.yank.clear();
        self.last_size = terminal.size().unwrap_or(self.last_size);
        self.note_activity();
        self.request_completion();
        Ok(())
    }

    fn finish(
        &mut self,
        renderer: &mut Renderer,
        terminal: &mut Terminal,
        reason: ExitReason,
    ) -> io::Result<()> {
        let size = terminal.size().unwrap_or(self.last_size);
        let (restore_position, replacement) = match reason {
            ExitReason::Accepted | ExitReason::Interrupted => (
                self.restore_prompt_position(size, crate::scene::TRANSIENT_PROMPT),
                Some(Scene::for_transient(self, size).render()),
            ),
            ExitReason::DelegateUp
            | ExitReason::DelegateDown
            | ExitReason::DelegateTab
            | ExitReason::DelegateEof => (self.restore_position(size), None),
        };
        renderer.finish(terminal, restore_position, replacement)
    }
}
