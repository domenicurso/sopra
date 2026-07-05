use crate::{PromptConfig, RenderConfig, ShellSnapshot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionConfig {
    pub prompt: PromptConfig,
    pub initial_buffer: String,
    pub initial_cursor: usize,
    pub shell: ShellSnapshot,
    pub render: RenderConfig,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            prompt: PromptConfig::default(),
            initial_buffer: String::new(),
            initial_cursor: 0,
            shell: ShellSnapshot::default(),
            render: RenderConfig::default(),
        }
    }
}
