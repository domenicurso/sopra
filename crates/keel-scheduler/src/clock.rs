use std::time::{Duration, Instant};

use crate::CursorBlink;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidationReason {
    HostRedisplay,
    Resize,
    Content,
    Accept,
    Cancel,
    Animation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameDecision {
    RenderNow,
    Wait(Duration),
    Idle,
}

#[derive(Debug, Clone)]
pub struct FrameClock {
    pending: bool,
    next_frame: Option<Instant>,
    animation_interval: Option<Duration>,
    editing: bool,
    animation_started: Option<Instant>,
    cursor_blink: CursorBlink,
}

impl Default for FrameClock {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameClock {
    pub const fn new() -> Self {
        Self {
            pending: false,
            next_frame: None,
            animation_interval: None,
            editing: false,
            animation_started: None,
            cursor_blink: CursorBlink::new(Duration::from_millis(1_800)),
        }
    }

    pub fn set_editing(&mut self, editing: bool, now: Instant) {
        let was_editing = self.editing;
        self.editing = editing;
        if editing {
            if !was_editing || self.animation_started.is_none() {
                self.animation_started = Some(now);
            }
            if let Some(interval) = self.animation_interval {
                self.next_frame = Some(now + interval);
            }
        } else {
            self.pending = false;
            self.next_frame = None;
            self.animation_started = None;
        }
    }

    pub fn set_cursor_blink(&mut self, blink: CursorBlink) {
        self.cursor_blink = blink;
    }

    pub fn cursor_opacity(&self, now: Instant) -> f32 {
        if !self.editing {
            return 1.0;
        }
        self.animation_started
            .map(|started| {
                self.cursor_blink
                    .opacity_at(now.saturating_duration_since(started))
            })
            .unwrap_or(1.0)
    }

    pub fn invalidate(&mut self, reason: InvalidationReason, now: Instant) {
        if matches!(reason, InvalidationReason::Animation) && !self.editing {
            return;
        }
        self.pending = true;
        if matches!(reason, InvalidationReason::Animation) {
            self.next_frame = self.animation_interval.map(|interval| now + interval);
        } else {
            self.next_frame = Some(now);
        }
    }

    pub fn set_animation_interval(&mut self, interval: Option<Duration>, now: Instant) {
        self.animation_interval = interval;
        if self.editing {
            if let Some(interval) = interval {
                self.next_frame = Some(now + interval);
            } else if !self.pending {
                self.next_frame = None;
            }
        } else if !self.pending {
            self.next_frame = None;
        }
    }

    pub fn decision(&self, now: Instant) -> FrameDecision {
        if !self.pending {
            return match self.next_frame {
                Some(next) if next <= now => FrameDecision::RenderNow,
                Some(next) => FrameDecision::Wait(next.saturating_duration_since(now)),
                None => FrameDecision::Idle,
            };
        }

        match self.next_frame {
            Some(next) if next > now => FrameDecision::Wait(next - now),
            _ => FrameDecision::RenderNow,
        }
    }

    pub fn rendered(&mut self, now: Instant) {
        self.pending = false;
        self.next_frame = self
            .editing
            .then_some(self.animation_interval)
            .flatten()
            .map(|interval| now + interval);
    }

    pub fn pending(&self) -> bool {
        self.pending
    }
}
