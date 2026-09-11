use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidationReason {
    Typing,
    CursorMoved,
    Resize,
    Lifecycle,
    Timer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameDecision {
    Immediate,
    At(Instant),
    Idle,
}

#[derive(Debug, Clone, Copy)]
pub struct FrameClock {
    blink_period: Duration,
    last_frame: Option<Instant>,
    editing: bool,
}

impl FrameClock {
    pub fn new(blink_period: Duration) -> Self {
        Self {
            blink_period,
            last_frame: None,
            editing: false,
        }
    }

    pub fn set_editing(&mut self, editing: bool, now: Instant) -> FrameDecision {
        self.editing = editing;
        if editing {
            self.last_frame = Some(now);
            FrameDecision::Immediate
        } else {
            self.last_frame = None;
            FrameDecision::Idle
        }
    }

    pub fn invalidate(&mut self, reason: InvalidationReason, now: Instant) -> FrameDecision {
        if matches!(reason, InvalidationReason::Timer) && !self.editing {
            return FrameDecision::Idle;
        }
        self.last_frame = Some(now);
        FrameDecision::Immediate
    }

    pub fn next(&self, now: Instant) -> FrameDecision {
        if !self.editing {
            return FrameDecision::Idle;
        }

        match self.last_frame {
            Some(last) => FrameDecision::At(last + self.blink_period),
            None => FrameDecision::Immediate,
        }
        .normalize(now)
    }

    pub fn should_blink(&self, now: Instant) -> bool {
        self.editing
            && self
                .last_frame
                .is_some_and(|last| now.duration_since(last) >= self.blink_period)
    }
}

impl FrameDecision {
    fn normalize(self, now: Instant) -> Self {
        match self {
            Self::At(deadline) if deadline <= now => Self::Immediate,
            other => other,
        }
    }
}

impl Default for FrameClock {
    fn default() -> Self {
        Self::new(Duration::from_millis(500))
    }
}

#[cfg(test)]
mod tests {
    use super::{FrameClock, FrameDecision, InvalidationReason};
    use std::time::{Duration, Instant};

    #[test]
    fn idle_clock_does_not_schedule_timer_work() {
        let now = Instant::now();
        let mut clock = FrameClock::default();

        assert_eq!(
            clock.invalidate(InvalidationReason::Timer, now),
            FrameDecision::Idle
        );
        assert_eq!(clock.next(now), FrameDecision::Idle);
    }

    #[test]
    fn editing_clock_redraws_immediately_and_blinks_later() {
        let now = Instant::now();
        let mut clock = FrameClock::new(Duration::from_millis(100));

        assert_eq!(clock.set_editing(true, now), FrameDecision::Immediate);
        assert_eq!(
            clock.next(now),
            FrameDecision::At(now + Duration::from_millis(100))
        );
        assert!(!clock.should_blink(now + Duration::from_millis(99)));
        assert!(clock.should_blink(now + Duration::from_millis(100)));
    }
}
