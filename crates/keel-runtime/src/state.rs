use keel_core::{CanvasBuffer, TerminalOwnershipState, TerminalSize};
use keel_editor::EditorBuffer;
use keel_core::PromptSurface;
use keel_prompt::PromptLayout;

use crate::FrontendMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeState {
    pub mode: FrontendMode,
    pub terminal_ownership: TerminalOwnershipState,
    pub prompt: PromptLayout,
    pub terminal_size: TerminalSize,
    pub editor: EditorBuffer,
    pub cursor_alpha: u8,
    pub final_canvas: Option<CanvasBuffer>,
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            mode: FrontendMode::PromptEditing,
            terminal_ownership: TerminalOwnershipState::Frontend,
            prompt: PromptLayout {
                active_left: PromptSurface::default(),
                active_right: PromptSurface::default(),
                transient_left: PromptSurface::default(),
            },
            terminal_size: TerminalSize::default(),
            editor: EditorBuffer::default(),
            cursor_alpha: u8::MAX,
            final_canvas: None,
        }
    }
}
