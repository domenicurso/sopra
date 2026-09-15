mod editing;
mod view;

#[cfg(test)]
mod tests;

use std::{
    io,
    time::{Duration, Instant},
};

use crate::{
    input::{CursorPosition, Terminal, TerminalSize},
    render::Renderer,
    scene::Scene,
};

const FRAME_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExitReason {
    Accepted,
    Cancelled,
    Interrupted,
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
}

pub(crate) struct EditorState {
    original_buffer: String,
    buffer: String,
    cursor: usize,
    original_cursor: usize,
    prompt: String,
    anchor: CursorPosition,
    selected: usize,
    overlay_visible: bool,
    status: String,
    animation_started: Instant,
    last_size: TerminalSize,
}

impl EditorState {
    pub(crate) fn new(config: EditorConfig) -> Self {
        let cursor = editing::char_index_to_byte(&config.buffer, config.cursor_chars);
        Self {
            original_buffer: config.buffer.clone(),
            buffer: config.buffer,
            cursor,
            original_cursor: cursor,
            prompt: config.prompt,
            anchor: config.anchor,
            selected: 0,
            overlay_visible: true,
            status: "ready".to_string(),
            animation_started: Instant::now(),
            last_size: config.size,
        }
    }

    pub(crate) fn run(&mut self, terminal: &mut Terminal) -> io::Result<RunResult> {
        let mut renderer = Renderer::default();
        let mut next_frame = Instant::now();
        loop {
            let now = Instant::now();
            if now >= next_frame {
                self.paint_editor(now, terminal, &mut renderer)?;
                next_frame = now + FRAME_INTERVAL;
            }

            let wait = next_frame.saturating_duration_since(Instant::now());
            if terminal.poll(wait)? {
                if let Some(result) = self.handle_key(terminal.read_key()?) {
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
        if size != self.last_size {
            self.last_size = size;
            self.status = "resized".to_string();
        }
        size
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
            ExitReason::Cancelled => (self.restore_position(size), None),
        };
        renderer.finish(terminal, restore_position, replacement)
    }
}
