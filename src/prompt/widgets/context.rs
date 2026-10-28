use super::Widget;

struct TextWidget {
    token: &'static str,
    text: String,
}

impl Widget for TextWidget {
    fn token(&self) -> &'static str {
        self.token
    }
    fn render(&self) -> String {
        self.text.clone()
    }
}

pub(super) fn widgets() -> Vec<Box<dyn Widget>> {
    let status = std::env::var("SOPRA_WIDGET_STATUS")
        .ok()
        .and_then(|value| value.parse::<i32>().ok())
        .filter(|value| *value != 0)
        .map(|value| value.to_string())
        .unwrap_or_default();
    let duration = std::env::var("SOPRA_WIDGET_DURATION")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(format_duration)
        .unwrap_or_default();
    [
        ("{status}", status),
        ("{duration}", duration),
        ("{user}", env_text("SOPRA_WIDGET_USER")),
        ("{host}", env_text("SOPRA_WIDGET_HOST")),
        ("{shell}", "zsh".to_string()),
    ]
    .into_iter()
    .map(|(token, text)| Box::new(TextWidget { token, text }) as Box<dyn Widget>)
    .collect()
}

fn env_text(name: &str) -> String {
    std::env::var(name)
        .unwrap_or_default()
        .chars()
        .filter(|character| !character.is_control())
        .collect()
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
