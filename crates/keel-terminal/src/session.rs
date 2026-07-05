use std::io;

use keel_core::{InputEvent, RuntimeOutcome, SessionConfig};
use keel_render::{CanvasPatch, FrameScheduler, RedrawReason};
use keel_runtime::{KeelRuntime, RuntimeStep};

use crate::TerminalIo;

pub fn run_command_read_session<T: TerminalIo>(
    terminal: &mut T,
    config: SessionConfig,
) -> io::Result<RuntimeOutcome> {
    let mut scheduler = FrameScheduler::new(&config.render);
    let mut runtime = KeelRuntime::new(config, terminal.size()?);
    let mut canvas = runtime.compose_active_canvas();
    terminal.enter_frontend()?;
    terminal.resume_frontend(&canvas)?;

    loop {
        let event = terminal
            .read_event(scheduler.interval())?
            .unwrap_or(InputEvent::TimerTick);

        let redraw_reason = match event {
            InputEvent::Resize(_) => RedrawReason::Resize,
            InputEvent::FocusGained | InputEvent::FocusLost => RedrawReason::Focus,
            InputEvent::TimerTick => RedrawReason::Timer,
            _ => RedrawReason::StateChange,
        };

        match runtime.apply_event(event) {
            RuntimeStep::Continue { redraw } => {
                if redraw {
                    scheduler.mark_dirty(redraw_reason);
                }
                if scheduler.should_render() {
                    let next = runtime.compose_active_canvas();
                    if next != canvas {
                        terminal.draw(&CanvasPatch::between(Some(&canvas), &next))?;
                        canvas = next;
                    }
                }
            }
            RuntimeStep::Accepted(handoff) => {
                let next = runtime
                    .state()
                    .final_canvas
                    .clone()
                    .unwrap_or_else(|| runtime.compose_submitted_canvas());
                terminal.finalize_submission(&next, &CanvasPatch::between(Some(&canvas), &next))?;
                runtime.prepare_command_handoff();
                terminal.suspend_for_command()?;
                runtime.suspend();
                return Ok(RuntimeOutcome::Accepted(handoff));
            }
            RuntimeStep::Cancelled => {
                runtime.begin_recovery();
                terminal.clear_frontend(&canvas)?;
                return Ok(RuntimeOutcome::Cancelled);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, io};

    use super::run_command_read_session;
    use crate::TerminalIo;
    use keel_core::{CanvasBuffer, InputEvent, Key, RuntimeOutcome, SessionConfig, TerminalSize};
    use keel_render::CanvasPatch;

    #[derive(Debug)]
    struct StubTerminal {
        size: TerminalSize,
        events: VecDeque<InputEvent>,
        canvases: Vec<CanvasBuffer>,
        entered: bool,
        suspended: bool,
        cleared: bool,
    }

    impl StubTerminal {
        fn new(events: impl Into<VecDeque<InputEvent>>) -> Self {
            Self {
                size: TerminalSize {
                    columns: 32,
                    rows: 24,
                },
                events: events.into(),
                canvases: Vec::new(),
                entered: false,
                suspended: false,
                cleared: false,
            }
        }
    }

    impl TerminalIo for StubTerminal {
        fn size(&self) -> io::Result<TerminalSize> {
            Ok(self.size)
        }

        fn enter_frontend(&mut self) -> io::Result<()> {
            self.entered = true;
            Ok(())
        }

        fn read_event(&mut self, _timeout: std::time::Duration) -> io::Result<Option<InputEvent>> {
            Ok(self.events
                .pop_front()
                .or(Some(InputEvent::TimerTick)))
        }

        fn draw(&mut self, patch: &CanvasPatch) -> io::Result<()> {
            let previous = self
                .canvases
                .last()
                .cloned()
                .unwrap_or_else(|| CanvasBuffer::blank(self.size));
            let mut next = if patch.full_redraw {
                CanvasBuffer::blank(patch.next_size)
            } else {
                previous
            };
            next.size = patch.next_size;
            next.rows.resize_with(
                patch.next_size.rows as usize,
                || keel_core::CanvasRow::blank(patch.next_size.columns),
            );
            for row in &mut next.rows {
                row.cells.resize(
                    patch.next_size.columns as usize,
                    keel_core::CanvasCell::default(),
                );
            }

            for row in &patch.rows {
                if let Some(slot) = next.rows.get_mut(row.row as usize) {
                    *slot = row.cells.clone();
                }
            }

            next.cursor = patch.cursor;
            self.canvases.push(next);
            Ok(())
        }

        fn finalize_submission(&mut self, _canvas: &CanvasBuffer, patch: &CanvasPatch) -> io::Result<()> {
            self.draw(patch)
        }

        fn suspend_for_command(&mut self) -> io::Result<()> {
            self.suspended = true;
            Ok(())
        }

        fn resume_frontend(&mut self, canvas: &CanvasBuffer) -> io::Result<()> {
            self.canvases.push(canvas.clone());
            Ok(())
        }

        fn clear_frontend(&mut self, _canvas: &CanvasBuffer) -> io::Result<()> {
            self.cleared = true;
            Ok(())
        }
    }

    #[test]
    fn accepts_a_command_and_renders_submission() {
        let mut terminal = StubTerminal::new(VecDeque::from(vec![
            InputEvent::Key(Key::Char('p')),
            InputEvent::Key(Key::Char('w')),
            InputEvent::Key(Key::Char('d')),
            InputEvent::Key(Key::Enter),
        ]));

        let outcome = run_command_read_session(&mut terminal, SessionConfig::default()).unwrap();

        let RuntimeOutcome::Accepted(handoff) = outcome else {
            panic!("expected accepted outcome");
        };
        assert_eq!(handoff.command, "pwd".to_string());
        assert_eq!(
            terminal.canvases.last().unwrap().plain_text(),
            "> pwd".to_string()
        );
        assert!(terminal.entered);
        assert!(terminal.suspended);
        assert!(!terminal.cleared);
    }

    #[test]
    fn clears_the_region_on_cancel() {
        let mut terminal = StubTerminal::new(VecDeque::from(vec![InputEvent::Key(Key::Esc)]));

        let outcome = run_command_read_session(&mut terminal, SessionConfig::default()).unwrap();

        assert_eq!(outcome, RuntimeOutcome::Cancelled);
        assert!(terminal.cleared);
    }
}
