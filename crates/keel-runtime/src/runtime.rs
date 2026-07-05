use std::f32::consts::PI;

use keel_core::{InputEvent, SessionConfig, SurfaceFrame, TerminalSize};
use keel_editor::{EditResult, EditorBuffer};
use keel_prompt::PromptRuntime;
use keel_render::Renderer;

use crate::{FrontendMode, RuntimeState, RuntimeStep};

const MIN_CURSOR_ALPHA: u8 = 72;

#[derive(Debug, Default)]
pub struct KeelRuntime {
    renderer: Renderer,
    state: RuntimeState,
    frame_interval_ms: u64,
    cursor_blink_ms: u64,
    cursor_animation_ms: u64,
    cursor_elapsed_ms: u64,
}

impl KeelRuntime {
    pub fn new(config: SessionConfig, terminal_size: TerminalSize) -> Self {
        let prompt_runtime = PromptRuntime;
        let prompt = prompt_runtime.layout(&config.prompt, &config.shell);

        Self {
            renderer: Renderer,
            state: RuntimeState {
                mode: FrontendMode::PromptEditing,
                prompt,
                terminal_size,
                editor: EditorBuffer::from_parts(config.initial_buffer, config.initial_cursor),
                cursor_alpha: u8::MAX,
            },
            frame_interval_ms: config.render.frame_interval_ms.max(1),
            cursor_blink_ms: config.render.cursor_blink_ms.max(1),
            cursor_animation_ms: config.render.cursor_animation_ms,
            cursor_elapsed_ms: 0,
        }
    }

    pub fn state(&self) -> &RuntimeState {
        &self.state
    }

    pub fn live_frame(&self) -> SurfaceFrame {
        self.renderer.render_active_with_cursor(
            &self.state.prompt.active_left,
            &self.state.prompt.active_right,
            &self.state.editor.snapshot(),
            self.state.terminal_size,
            self.state.cursor_alpha,
        )
    }

    pub fn submitted_frame(&self) -> SurfaceFrame {
        self.renderer.render_submitted(
            &self.state.prompt.transient_left,
            &self.state.editor.snapshot(),
            self.state.terminal_size,
        )
    }

    pub fn apply_event(&mut self, event: InputEvent) -> RuntimeStep {
        match event {
            InputEvent::Key(key) => match self.state.editor.apply_key(key) {
                EditResult::Continue => {
                    self.reset_cursor_blink();
                    RuntimeStep::Continue { redraw: true }
                }
                EditResult::Submit(command) => {
                    self.state.mode = FrontendMode::CommandSubmission;
                    RuntimeStep::Accepted(command)
                }
                EditResult::Cancel => {
                    self.state.mode = FrontendMode::Cancelled;
                    RuntimeStep::Cancelled
                }
            },
            InputEvent::Paste(text) => {
                if matches!(self.state.mode, FrontendMode::PromptEditing) {
                    self.state.editor.insert_str(&text);
                    self.reset_cursor_blink();
                } else {
                    self.state.mode = FrontendMode::Recovery;
                }

                RuntimeStep::Continue { redraw: true }
            }
            InputEvent::Resize(terminal_size) => {
                self.state.terminal_size = terminal_size;
                self.reset_cursor_blink();
                RuntimeStep::Continue { redraw: true }
            }
            InputEvent::FocusGained => {
                self.reset_cursor_blink();
                RuntimeStep::Continue { redraw: true }
            }
            InputEvent::FocusLost => RuntimeStep::Continue { redraw: false },
            InputEvent::TimerTick => {
                if matches!(self.state.mode, FrontendMode::PromptEditing) {
                    let previous_alpha = self.state.cursor_alpha;
                    self.cursor_elapsed_ms = self
                        .cursor_elapsed_ms
                        .saturating_add(self.frame_interval_ms);
                    self.state.cursor_alpha = self.compute_cursor_alpha();

                    if self.state.cursor_alpha != previous_alpha {
                        return RuntimeStep::Continue { redraw: true };
                    }
                }
                RuntimeStep::Continue { redraw: false }
            }
        }
    }

    pub fn prepare_command_handoff(&mut self) {
        self.state.mode = FrontendMode::CommandExecutionHandoff;
    }

    pub fn begin_recovery(&mut self) {
        self.state.mode = FrontendMode::Recovery;
    }

    fn reset_cursor_blink(&mut self) {
        self.cursor_elapsed_ms = 0;
        self.state.cursor_alpha = u8::MAX;
    }

    fn compute_cursor_alpha(&self) -> u8 {
        let blink_ms = self.cursor_blink_ms.max(1);
        let animation_ms = self.cursor_animation_ms.min(blink_ms);

        if animation_ms == 0 {
            return u8::MAX;
        }

        if self.cursor_elapsed_ms < blink_ms {
            return u8::MAX;
        }

        let cycle_offset = (self.cursor_elapsed_ms - blink_ms) % blink_ms;
        if cycle_offset >= animation_ms {
            u8::MAX
        } else {
            pulse_alpha(cycle_offset, animation_ms)
        }
    }
}

fn pulse_alpha(elapsed_ms: u64, duration_ms: u64) -> u8 {
    if duration_ms <= 1 {
        return MIN_CURSOR_ALPHA;
    }

    let progress = (elapsed_ms.min(duration_ms) as f32 / duration_ms as f32).clamp(0.0, 1.0);
    let wave = (progress * 2.0 * PI).cos();
    let normalized = ((wave + 1.0) * 0.5).clamp(0.0, 1.0);
    let min = MIN_CURSOR_ALPHA as f32;
    let max = u8::MAX as f32;
    (min + ((max - min) * normalized)).round() as u8
}

#[cfg(test)]
mod tests {
    use super::KeelRuntime;
    use crate::{FrontendMode, RuntimeStep};
    use keel_core::{InputEvent, Key, SessionConfig, TerminalSize};

    fn test_runtime() -> KeelRuntime {
        KeelRuntime::new(
            SessionConfig::default(),
            TerminalSize {
                columns: 32,
                rows: 24,
            },
        )
    }

    #[test]
    fn accepts_a_command_after_editing() {
        let mut runtime = test_runtime();

        assert_eq!(
            runtime.apply_event(InputEvent::Key(Key::Char('p'))),
            RuntimeStep::Continue { redraw: true }
        );
        assert_eq!(
            runtime.apply_event(InputEvent::Key(Key::Char('w'))),
            RuntimeStep::Continue { redraw: true }
        );
        assert_eq!(
            runtime.apply_event(InputEvent::Key(Key::Char('d'))),
            RuntimeStep::Continue { redraw: true }
        );
        assert_eq!(
            runtime.apply_event(InputEvent::Key(Key::Enter)),
            RuntimeStep::Accepted("pwd".to_string())
        );
        assert_eq!(runtime.state().mode, FrontendMode::CommandSubmission);

        runtime.prepare_command_handoff();
        assert_eq!(
            runtime.state().mode,
            FrontendMode::CommandExecutionHandoff
        );
        assert_eq!(runtime.submitted_frame().plain_text(), "> pwd");
    }

    #[test]
    fn resize_updates_runtime_state() {
        let mut runtime = test_runtime();

        assert_eq!(
            runtime.apply_event(InputEvent::Resize(TerminalSize {
                columns: 100,
                rows: 40,
            })),
            RuntimeStep::Continue { redraw: true }
        );
        assert_eq!(
            runtime.state().terminal_size,
            TerminalSize {
                columns: 100,
                rows: 40,
            }
        );
    }

    #[test]
    fn cancel_transitions_to_cancelled_mode() {
        let mut runtime = test_runtime();

        assert_eq!(
            runtime.apply_event(InputEvent::Key(Key::Esc)),
            RuntimeStep::Cancelled
        );
        assert_eq!(runtime.state().mode, FrontendMode::Cancelled);
    }

    #[test]
    fn timer_ticks_only_redraw_on_blink_boundary() {
        let mut runtime = test_runtime();

        for _ in 0..75 {
            assert_eq!(
                runtime.apply_event(InputEvent::TimerTick),
                RuntimeStep::Continue { redraw: false }
            );
            assert_eq!(runtime.state().cursor_alpha, u8::MAX);
        }

        assert_eq!(
            runtime.apply_event(InputEvent::TimerTick),
            RuntimeStep::Continue { redraw: true }
        );
        assert!(runtime.state().cursor_alpha < u8::MAX);

        assert_eq!(
            runtime.apply_event(InputEvent::TimerTick),
            RuntimeStep::Continue { redraw: true }
        );
        assert!(runtime.state().cursor_alpha < u8::MAX);
    }

    #[test]
    fn editing_resets_cursor_blink_phase() {
        let mut runtime = test_runtime();

        for _ in 0..76 {
            let _ = runtime.apply_event(InputEvent::TimerTick);
        }
        assert!(runtime.state().cursor_alpha < u8::MAX);

        assert_eq!(
            runtime.apply_event(InputEvent::Key(Key::Char('x'))),
            RuntimeStep::Continue { redraw: true }
        );
        assert_eq!(runtime.state().cursor_alpha, u8::MAX);

        for _ in 0..75 {
            assert_eq!(
                runtime.apply_event(InputEvent::TimerTick),
                RuntimeStep::Continue { redraw: false }
            );
        }

        assert_eq!(
            runtime.apply_event(InputEvent::TimerTick),
            RuntimeStep::Continue { redraw: true }
        );
        assert!(runtime.state().cursor_alpha < u8::MAX);
    }
}
