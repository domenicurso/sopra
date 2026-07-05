use std::time::Duration;

use keel_core::RenderConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedrawReason {
    StateChange,
    Resize,
    Focus,
    Timer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameScheduler {
    dirty: bool,
    interval: Duration,
}

impl FrameScheduler {
    pub fn new(config: &RenderConfig) -> Self {
        Self {
            dirty: true,
            interval: Duration::from_millis(config.frame_interval_ms),
        }
    }

    pub fn mark_dirty(&mut self, _reason: RedrawReason) {
        self.dirty = true;
    }

    pub fn should_render(&mut self) -> bool {
        let dirty = self.dirty;
        self.dirty = false;
        dirty
    }

    pub fn interval(&self) -> Duration {
        self.interval
    }
}
