use std::{
    io,
    time::{Duration, Instant},
};

use super::{EditorState, ExitReason, RunResult};
use crate::{
    input::{Key, Terminal, TerminalSize},
    render::Renderer,
    scene::Scene,
};

const FRAME_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / 60);

impl EditorState {
    pub(crate) fn run(&mut self, terminal: &mut Terminal) -> io::Result<RunResult> {
        let mut renderer = Renderer::editor();
        let mut resize_epoch = terminal.resize_epoch();
        let mut next_frame = Instant::now();
        loop {
            self.poll_completion();
            let now = Instant::now();
            if now >= next_frame {
                self.paint_editor(now, terminal, &mut renderer)?;
                next_frame = now + FRAME_INTERVAL;
            }

            let wait = next_frame.saturating_duration_since(Instant::now());
            let result = if terminal.poll(wait)? {
                let key = terminal.read_key()?;
                next_frame = Instant::now();
                self.handle_input(key, terminal, &mut renderer, &mut resize_epoch)?
            } else {
                None
            };
            if let Some(result) = result {
                self.finish(&mut renderer, terminal, result.reason)?;
                if result.reason == ExitReason::Interrupted {
                    self.start_fresh_line(terminal)?;
                    renderer = Renderer::editor();
                    next_frame = Instant::now();
                    continue;
                }
                return Ok(result);
            }
            let latest_resize_epoch = terminal.resize_epoch();
            if latest_resize_epoch != resize_epoch {
                resize_epoch = latest_resize_epoch;
                next_frame = Instant::now();
            }
        }
    }

    fn handle_input(
        &mut self,
        key: Key,
        terminal: &mut Terminal,
        renderer: &mut Renderer,
        resize_epoch: &mut usize,
    ) -> io::Result<Option<RunResult>> {
        if key == Key::Cancel && self.copy_selection(terminal)? {
            return Ok(None);
        }
        if let Key::Mouse(event) = key {
            let latest = terminal.resize_epoch();
            if latest != *resize_epoch {
                *resize_epoch = latest;
                self.paint_editor(Instant::now(), terminal, renderer)?;
            }
            self.handle_mouse(event, renderer.origin_row());
            return Ok(None);
        }
        Ok(self.handle_key(key))
    }

    fn paint_editor(
        &mut self,
        now: Instant,
        terminal: &mut Terminal,
        renderer: &mut Renderer,
    ) -> io::Result<()> {
        (self.prompt, self.rprompt) = self.widgets.render();
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

    fn start_fresh_line(&mut self, terminal: &mut Terminal) -> io::Result<()> {
        terminal.write_all(b"\r\n")?;
        terminal.flush()?;
        self.buffer.clear();
        self.text_selection.clear();
        self.cursor = 0;
        self.selected = None;
        self.completion_armed = false;
        self.completion_suspended = self.completion.is_none();
        self.overlay_visible = self.completion.is_some();
        self.suggestion_scroll = 0;
        self.completion_source.clear();
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
                self.restore_prompt_position(size, self.transient_prompt()),
                Some(Scene::for_transient(self, size).render()),
            ),
            ExitReason::ChangeDirectory => {
                (crate::input::CursorPosition { row: 0, column: 0 }, None)
            }
            ExitReason::DelegateUp
            | ExitReason::DelegateDown
            | ExitReason::DelegateTab
            | ExitReason::DelegateEof => (self.restore_position(size), None),
        };
        renderer.finish(terminal, restore_position, replacement)
    }
}
