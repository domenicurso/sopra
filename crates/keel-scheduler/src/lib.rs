use std::time::{Duration, Instant};

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
        }
    }

    pub fn set_editing(&mut self, editing: bool, now: Instant) {
        self.editing = editing;
        if editing {
            if let Some(interval) = self.animation_interval {
                self.next_frame = Some(now + interval);
            }
        } else {
            self.pending = false;
            self.next_frame = None;
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_clock_does_not_request_frames() {
        let clock = FrameClock::new();
        assert_eq!(clock.decision(Instant::now()), FrameDecision::Idle);
    }

    #[test]
    fn typing_and_resize_render_immediately() {
        let now = Instant::now();
        let mut clock = FrameClock::new();
        clock.invalidate(InvalidationReason::Content, now);
        assert_eq!(clock.decision(now), FrameDecision::RenderNow);
        clock.rendered(now);
        clock.invalidate(InvalidationReason::Resize, now);
        assert_eq!(clock.decision(now), FrameDecision::RenderNow);
    }

    #[test]
    fn animation_waits_until_the_next_tick() {
        let now = Instant::now();
        let mut clock = FrameClock::new();
        clock.set_editing(true, now);
        clock.set_animation_interval(Some(Duration::from_millis(100)), now);
        assert_eq!(
            clock.decision(now),
            FrameDecision::Wait(Duration::from_millis(100))
        );
        assert_eq!(
            clock.decision(now + Duration::from_millis(100)),
            FrameDecision::RenderNow
        );
    }

    #[test]
    fn rendering_clears_pending_work_but_keeps_animation_alive() {
        let now = Instant::now();
        let mut clock = FrameClock::new();
        clock.set_editing(true, now);
        clock.set_animation_interval(Some(Duration::from_millis(50)), now);
        clock.invalidate(InvalidationReason::Animation, now);
        clock.rendered(now + Duration::from_millis(50));
        assert!(!clock.pending());
        assert_eq!(
            clock.decision(now + Duration::from_millis(50)),
            FrameDecision::Wait(Duration::from_millis(50))
        );
    }

    #[test]
    fn animation_is_ignored_when_the_editor_is_idle() {
        let now = Instant::now();
        let mut clock = FrameClock::new();
        clock.set_animation_interval(Some(Duration::from_millis(50)), now);
        clock.invalidate(InvalidationReason::Animation, now);
        assert_eq!(clock.decision(now), FrameDecision::Idle);
    }
}
