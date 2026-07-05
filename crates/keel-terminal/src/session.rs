use std::io;

use keel_core::{InputEvent, RuntimeOutcome, SessionConfig};
use keel_render::{FramePatch, FrameScheduler, RedrawReason};
use keel_runtime::{KeelRuntime, RuntimeStep};

use crate::TerminalIo;

pub fn run_command_read_session<T: TerminalIo>(
    terminal: &mut T,
    config: SessionConfig,
) -> io::Result<RuntimeOutcome> {
    let mut scheduler = FrameScheduler::new(&config.render);
    let mut runtime = KeelRuntime::new(config, terminal.size()?);
    let mut frame = runtime.live_frame();
    terminal.apply_patch(&FramePatch::between(None, &frame))?;

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
                    let next = runtime.live_frame();
                    if next != frame {
                        terminal.apply_patch(&FramePatch::between(Some(&frame), &next))?;
                        frame = next;
                    }
                }
            }
            RuntimeStep::Accepted(command) => {
                let next = runtime.submitted_frame();
                terminal.apply_patch(&FramePatch::between(Some(&frame), &next))?;
                runtime.prepare_command_handoff();
                return Ok(RuntimeOutcome::Accepted(command));
            }
            RuntimeStep::Cancelled => {
                runtime.begin_recovery();
                terminal.clear_render_region(frame.lines.len() as u16)?;
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
    use keel_core::{InputEvent, Key, RuntimeOutcome, SessionConfig, SurfaceLine, TerminalSize};
    use keel_render::FramePatch;

    #[derive(Debug)]
    struct StubTerminal {
        size: TerminalSize,
        events: VecDeque<InputEvent>,
        frames: Vec<keel_core::SurfaceFrame>,
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
                frames: Vec::new(),
                cleared: false,
            }
        }
    }

    impl TerminalIo for StubTerminal {
        fn size(&self) -> io::Result<TerminalSize> {
            Ok(self.size)
        }

        fn read_event(&mut self, _timeout: std::time::Duration) -> io::Result<Option<InputEvent>> {
            Ok(self.events
                .pop_front()
                .or(Some(InputEvent::TimerTick)))
        }

        fn apply_patch(&mut self, patch: &FramePatch) -> io::Result<()> {
            let previous = self.frames.last().cloned().unwrap_or_default();
            let mut next = keel_core::SurfaceFrame {
                lines: previous.lines.clone(),
                cursor: patch.cursor,
            };

            next.lines
                .resize(patch.next_height as usize, SurfaceLine::default());

            for line in &patch.lines {
                if let Some(slot) = next.lines.get_mut(line.row as usize) {
                    *slot = line.line.clone();
                }
            }

            self.frames.push(next);
            Ok(())
        }

        fn clear_render_region(&mut self, _previous_height: u16) -> io::Result<()> {
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

        assert_eq!(outcome, RuntimeOutcome::Accepted("pwd".to_string()));
        assert_eq!(
            terminal.frames.last().unwrap().plain_text(),
            "> pwd".to_string()
        );
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
