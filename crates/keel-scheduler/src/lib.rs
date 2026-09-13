use std::time::{Duration, Instant};

const DEFAULT_CURSOR_BLINK_PERIOD: Duration = Duration::from_millis(1_800);
const MIN_CURSOR_OPACITY: f32 = 0.4;

/// A cursor visibility cycle with a smooth transition at each edge.
///
/// The returned opacity is one at the beginning of a cycle, eases to a dimmed
/// state at the midpoint, and eases back to one by the end. Keeping the sample
/// continuous lets a renderer produce real intermediate frames instead of
/// treating blinking as a boolean state change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorBlink {
    period: Duration,
}

impl CursorBlink {
    pub const fn new(period: Duration) -> Self {
        Self { period }
    }

    pub const fn period(self) -> Duration {
        self.period
    }

    pub fn opacity_at(self, elapsed: Duration) -> f32 {
        if self.period.is_zero() {
            return 1.0;
        }

        let period = self.period.as_secs_f64();
        let phase = (elapsed.as_secs_f64() % period) / period;
        let half_cycle = phase * 2.0;
        let visibility = if half_cycle <= 1.0 {
            1.0 - smoothstep(half_cycle as f32)
        } else {
            smoothstep((half_cycle - 1.0) as f32)
        };
        MIN_CURSOR_OPACITY + visibility * (1.0 - MIN_CURSOR_OPACITY)
    }
}

impl Default for CursorBlink {
    fn default() -> Self {
        Self::new(DEFAULT_CURSOR_BLINK_PERIOD)
    }
}

fn smoothstep(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

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
            cursor_blink: CursorBlink::new(DEFAULT_CURSOR_BLINK_PERIOD),
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

    #[test]
    fn cursor_blink_has_intermediate_opacity_values() {
        let blink = CursorBlink::new(Duration::from_millis(1_000));

        assert_eq!(blink.opacity_at(Duration::ZERO), 1.0);
        assert_eq!(
            blink.opacity_at(Duration::from_millis(500)),
            MIN_CURSOR_OPACITY
        );
        assert_eq!(blink.opacity_at(Duration::from_millis(1_000)), 1.0);
        assert!(blink.opacity_at(Duration::from_millis(125)) > MIN_CURSOR_OPACITY);
        assert!(blink.opacity_at(Duration::from_millis(125)) < 1.0);
        assert!(blink.opacity_at(Duration::from_millis(625)) > MIN_CURSOR_OPACITY);
        assert!(blink.opacity_at(Duration::from_millis(625)) < 1.0);
    }

    #[test]
    fn editing_clock_exposes_the_interpolated_cursor_opacity() {
        let now = Instant::now();
        let mut clock = FrameClock::new();
        clock.set_cursor_blink(CursorBlink::new(Duration::from_millis(1_000)));
        clock.set_editing(true, now);

        let opacity = clock.cursor_opacity(now + Duration::from_millis(125));
        assert!(opacity > 0.0 && opacity < 1.0);
    }

    #[test]
    fn refreshing_editing_state_does_not_restart_the_cursor_animation() {
        let now = Instant::now();
        let mut clock = FrameClock::new();
        clock.set_editing(true, now);
        clock.set_editing(true, now + Duration::from_millis(125));

        let opacity = clock.cursor_opacity(now + Duration::from_millis(125));
        assert!(opacity > 0.0 && opacity < 1.0);
    }
}
