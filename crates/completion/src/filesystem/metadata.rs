use std::time::{Duration, SystemTime};

pub(super) fn file_age(modified: SystemTime) -> Option<String> {
    let age = SystemTime::now().duration_since(modified).ok()?;
    Some(format!("{} ago", age_label(age)))
}

fn age_label(age: Duration) -> String {
    let seconds = age.as_secs();
    let (value, unit) = match seconds {
        0..=59 => (seconds, "second"),
        60..=3_599 => (seconds / 60, "minute"),
        3_600..=86_399 => (seconds / 3_600, "hour"),
        86_400..=2_591_999 => (seconds / 86_400, "day"),
        2_592_000..=31_535_999 => (seconds / 2_592_000, "month"),
        _ => (seconds / 31_536_000, "year"),
    };
    format!("{value} {unit}{}", if value != 1 { "s" } else { "" })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::age_label;

    #[test]
    fn ages_use_compact_popup_labels() {
        assert_eq!(age_label(Duration::from_secs(60)), "1 minute");
        assert_eq!(age_label(Duration::from_secs(6 * 2_592_000)), "6 months");
    }
}
