use super::Widget;

struct Duration(String);

impl Widget for Duration {
    fn token(&self) -> &str { "{duration}" }
    fn render(&self) -> String { self.0.clone() }
}

pub(super) fn create(_context: &super::WidgetContext<'_>) -> Box<dyn Widget> {
    Box::new(Duration(sopra_shell_context::var("SOPRA_WIDGET_DURATION").ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(format_duration).unwrap_or_default()))
}
fn format_duration(seconds: f64) -> String {
    if seconds < 0.001 {
        format!("{:.1}ms", seconds * 1000.0)
    } else if seconds < 1.0 {
        format!("{:.0}ms", seconds * 1000.0)
    } else if seconds < 60.0 {
        format!("{seconds:.1}s")
    } else {
        let rounded = seconds.round() as u64;
        format!("{}m {}s", rounded / 60, rounded % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::format_duration;
    #[test]
    fn duration_uses_readable_units() {
        assert_eq!(format_duration(0.000_123), "0.1ms");
        assert_eq!(format_duration(0.000_876), "0.9ms");
        assert_eq!(format_duration(0.001), "1ms");
        assert_eq!(format_duration(0.125), "125ms");
        assert_eq!(format_duration(2.5), "2.5s");
        assert_eq!(format_duration(62.0), "1m 2s");
        assert_eq!(format_duration(119.6), "2m 0s");
    }
}
