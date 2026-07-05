use keel_core::TerminalSize;
use keel_editor::EditorBuffer;
use keel_prompt::PromptLayout;

use crate::FrontendMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeState {
    pub mode: FrontendMode,
    pub prompt: PromptLayout,
    pub terminal_size: TerminalSize,
    pub editor: EditorBuffer,
    pub cursor_alpha: u8,
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            mode: FrontendMode::PromptEditing,
            prompt: PromptLayout {
                active_left: String::new(),
                active_right: String::new(),
                transient_left: String::new(),
            },
            terminal_size: TerminalSize::default(),
            editor: EditorBuffer::default(),
            cursor_alpha: u8::MAX,
        }
    }
}
