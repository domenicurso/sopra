use std::time::Duration;

const DEFAULT_CURSOR_BLINK_PERIOD: Duration = Duration::from_millis(1_800);
pub(crate) const MIN_CURSOR_OPACITY: f32 = 0.4;

/// A cursor visibility cycle with a smooth transition at each edge.
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
