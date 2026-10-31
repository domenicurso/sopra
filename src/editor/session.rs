#[path = "session_loop.rs"]
mod lifecycle;
use std::{io, time::Instant};

use super::{EditorState, ExitReason, RunResult};
use crate::{
    actions::Action,
    input::{Terminal, TerminalSize},
    render::Renderer,
    scene::Scene,
};

impl EditorState {
    pub(crate) fn run(
        &mut self,
        terminal: &mut Terminal,
        syntax: &mut super::submission::Checker,
    ) -> io::Result<RunResult> {
        syntax.cancel();
        let mut frame = lifecycle::Loop::new(terminal);
        loop {
            if let Some(result) = frame.step(self, terminal, syntax)? {
                self.finish(&mut frame.renderer, terminal, result.reason)?;
                if result.reason == ExitReason::Interrupted {
                    self.start_fresh_line(terminal)?;
                    frame = lifecycle::Loop::new(terminal);
                } else {
                    return Ok(result);
                }
            }
        }
    }

    fn poll_actions(
        &mut self,
        syntax: &mut super::submission::Checker,
    ) -> io::Result<Option<RunResult>> {
        self.poll_completion();
        if let Some(sopra::Action::ChangeDirectory(directory)) = self.widgets.poll_action() {
            let mut result = self.result(ExitReason::ChangeDirectory);
            result.directory = Some(directory);
            return Ok(Some(result));
        }
        if let Some(incomplete) = syntax.poll(&self.buffer)? {
            if incomplete {
                self.continue_input();
            } else {
                return Ok(Some(self.result(ExitReason::Accepted)));
            }
        }
        Ok(None)
    }

    fn handle_input(
        &mut self,
        key: Action,
        terminal: &mut Terminal,
        renderer: &mut Renderer,
        resize_epoch: &mut usize,
    ) -> io::Result<Option<RunResult>> {
        if key == Action::Cancel && self.copy_selection(terminal)? {
            return Ok(None);
        }
        if let Action::Mouse(event) = key {
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

    fn continue_input(&mut self) {
        self.buffer.push('\n');
        self.cursor = self.buffer.len();
        self.text_selection.clear();
        self.note_activity();
        self.request_completion();
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
        self.completions.focus.select(None);
        self.completions.focus.set_armed(false);
        self.completions
            .set_suspended(self.completions.engine.is_none());
        self.completions.visible = self.completions.engine.is_some();
        self.completions.scroll = 0;
        self.completions.source.clear();
        self.completions.items.clear();
        self.completions.key.clear();
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
            ExitReason::Accepted => (
                self.restore_prompt_position(size, &crate::prompt::parse(self.transient_prompt())),
                Some(Scene::for_transient(self, size).render()),
            ),
            ExitReason::Interrupted => {
                let frame = Scene::for_transient(self, size).render();
                (frame.cursor, Some(frame))
            }
            ExitReason::ChangeDirectory => {
                (crate::input::CursorPosition { row: 0, column: 0 }, None)
            }
            ExitReason::DelegateTab | ExitReason::DelegateEof => {
                (self.restore_position(size), None)
            }
        };
        renderer.finish(terminal, restore_position, replacement)
    }
}
