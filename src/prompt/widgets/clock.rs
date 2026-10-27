use std::{
    ffi::{CStr, CString},
    time::{SystemTime, UNIX_EPOCH},
};

use super::Widget;

pub(super) struct Clock {
    format: CString,
}

impl Clock {
    pub(super) fn new() -> Self {
        let configured = std::env::var("SOPRA_CLOCK_FORMAT").unwrap_or_else(|_| "%H:%M:%S".into());
        Self {
            format: CString::new(configured).unwrap_or_default(),
        }
    }
}

impl Widget for Clock {
    fn token(&self) -> &'static str {
        "{clock}"
    }
    fn markup(&self) -> bool {
        true
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
        if unsafe { libc::localtime_r(&seconds, local.as_mut_ptr()) }.is_null() {
            return String::new();
        }
        // The successful localtime_r call initialized this value.
        let local = unsafe { local.assume_init() };
        let mut output = [0 as libc::c_char; 4096];
        // The format and tm are valid and strftime writes at most output.len() bytes.
        let len = unsafe {
            libc::strftime(
                output.as_mut_ptr(),
                output.len(),
                self.format.as_ptr(),
                &local,
            )
        };
        if len == 0 {
            return String::new();
        }
        // A successful strftime call writes a terminating NUL within the output buffer.
        unsafe { CStr::from_ptr(output.as_ptr()) }
            .to_string_lossy()
            .into_owned()
    }
}
