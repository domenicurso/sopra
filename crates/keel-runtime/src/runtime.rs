use std::f32::consts::PI;

use keel_core::{
    CanvasBuffer, CommandHandoff, CursorStyle, FrontendScene, InputEvent, SceneCursor,
    SessionConfig, TerminalOwnershipState, TerminalRelease, TerminalSize,
};
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
                terminal_ownership: TerminalOwnershipState::Frontend,
                prompt,
                terminal_size,
                editor: EditorBuffer::from_parts(config.initial_buffer, config.initial_cursor),
                cursor_alpha: u8::MAX,
                final_canvas: None,
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

    pub fn active_scene(&self) -> FrontendScene {
        FrontendScene {
            terminal_size: self.state.terminal_size,
            prompt_left: self.state.prompt.active_left.clone(),
            prompt_right: self.state.prompt.active_right.clone(),
            editor: self.state.editor.snapshot(),
            cursor: Some(SceneCursor {
                style: CursorStyle::Block,
                alpha: self.state.cursor_alpha,
                visible: true,
            }),
            overlay_anchor: None,
        }
    }

    pub fn submitted_scene(&self) -> FrontendScene {
        FrontendScene {
            terminal_size: self.state.terminal_size,
            prompt_left: self.state.prompt.transient_left.clone(),
            prompt_right: keel_core::PromptSurface::default(),
            editor: self.state.editor.snapshot(),
            cursor: None,
            overlay_anchor: None,
        }
    }

    pub fn compose_active_canvas(&self) -> CanvasBuffer {
        self.renderer.compose(&self.active_scene())
    }

    pub fn compose_submitted_canvas(&self) -> CanvasBuffer {
        self.renderer.compose(&self.submitted_scene())
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
                    let final_canvas = self.compose_submitted_canvas();
                    self.state.final_canvas = Some(final_canvas);
                    RuntimeStep::Accepted(CommandHandoff {
                        command,
                        terminal_release: TerminalRelease::SuspendFrontend,
                    })
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
        self.state.terminal_ownership = keel_core::TerminalOwnershipState::Suspended;
    }

    pub fn suspend(&mut self) {
        self.state.mode = FrontendMode::Suspended;
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
    use keel_core::{InputEvent, Key, SessionConfig, TerminalSize, TerminalOwnershipState};

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
            RuntimeStep::Accepted(keel_core::CommandHandoff {
                command: "pwd".to_string(),
                terminal_release: keel_core::TerminalRelease::SuspendFrontend,
            })
        );
        assert_eq!(runtime.state().mode, FrontendMode::CommandSubmission);

        runtime.prepare_command_handoff();
        assert_eq!(
            runtime.state().mode,
            FrontendMode::CommandExecutionHandoff
        );
        runtime.suspend();
        assert_eq!(runtime.state().mode, FrontendMode::Suspended);
        assert_eq!(
            runtime.state().terminal_ownership,
            TerminalOwnershipState::Suspended
        );
        assert_eq!(runtime.compose_submitted_canvas().plain_text(), "> pwd");
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
        let ticks_until_blink_change = runtime
            .cursor_blink_ms
            .div_ceil(runtime.frame_interval_ms) as usize
            + 1;

        for _ in 0..ticks_until_blink_change.saturating_sub(1) {
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
        let ticks_until_blink_change = runtime
            .cursor_blink_ms
            .div_ceil(runtime.frame_interval_ms) as usize
            + 1;

        for _ in 0..ticks_until_blink_change {
            let _ = runtime.apply_event(InputEvent::TimerTick);
        }
        assert!(runtime.state().cursor_alpha < u8::MAX);

        assert_eq!(
            runtime.apply_event(InputEvent::Key(Key::Char('x'))),
            RuntimeStep::Continue { redraw: true }
        );
        assert_eq!(runtime.state().cursor_alpha, u8::MAX);

        for _ in 0..ticks_until_blink_change.saturating_sub(1) {
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
