use std::time::{SystemTime, UNIX_EPOCH};

use super::Widget;

pub(super) struct Clock;

impl Widget for Clock {
    fn token(&self) -> &'static str {
        "{clock}"
    }

    fn render(&self) -> String {
        let Some(seconds) = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|value| libc::time_t::try_from(value.as_secs()).ok())
        else {
            return String::new();
        };
        let mut local = std::mem::MaybeUninit::<libc::tm>::uninit();
        // localtime_r initializes the provided tm on success and retains no pointers.
        let result = unsafe { libc::localtime_r(&seconds, local.as_mut_ptr()) };
        if result.is_null() {
            return String::new();
        }
        // A non-null result guarantees localtime_r initialized the tm above.
        let local = unsafe { local.assume_init() };
        format!(
            "{:02}:{:02}:{:02}",
            local.tm_hour, local.tm_min, local.tm_sec
        )
    }
}
