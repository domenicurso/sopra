use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderConfig {
    pub frame_interval_ms: u64,
    pub cursor_blink_ms: u64,
    pub cursor_animation_ms: u64,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            frame_interval_ms: 16,
            cursor_blink_ms: 1200,
            cursor_animation_ms: 180,
        }
    }
}
