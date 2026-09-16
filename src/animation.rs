use std::time::Duration;

const CURSOR_PERIOD: Duration = Duration::from_millis(1_800);
const MIN_CURSOR_OPACITY: f32 = 0.4;

pub(crate) fn cursor_opacity(elapsed: Duration) -> f32 {
    if CURSOR_PERIOD.is_zero() {
        return 1.0;
    }
    let phase = (elapsed.as_secs_f64() % CURSOR_PERIOD.as_secs_f64()) / CURSOR_PERIOD.as_secs_f64();
    let cycle = phase * 2.0;
    let visibility = if cycle <= 1.0 {
        1.0 - smoothstep(cycle as f32)
    } else {
        smoothstep((cycle - 1.0) as f32)
    };
    MIN_CURSOR_OPACITY + visibility * (1.0 - MIN_CURSOR_OPACITY)
}

fn smoothstep(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::cursor_opacity;

    #[test]
    fn cursor_opacity_has_a_smooth_visible_cycle() {
        let start = cursor_opacity(Duration::ZERO);
        let middle = cursor_opacity(Duration::from_millis(450));
        assert_eq!(start, 1.0);
        assert!(middle < start && middle > 0.4);
    }
}
