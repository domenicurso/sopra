use super::control::Control;
use std::sync::{Condvar, Mutex};

static ACTIVE: (Mutex<usize>, Condvar) = (Mutex::new(0), Condvar::new());
const LIMIT: usize = 6;

pub(super) struct Permit;

impl Permit {
    pub(super) fn acquire(control: &Control) -> Option<Self> {
        let (lock, available) = &ACTIVE;
        let mut active = lock.lock().ok()?;
        while *active >= LIMIT {
            if control.stopped() {
                return None;
            }
            active = available
                .wait_timeout(active, std::time::Duration::from_millis(5))
                .ok()?
                .0;
        }
        if control.stopped() {
            return None;
        }
        *active += 1;
        Some(Self)
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        let (lock, available) = &ACTIVE;
        if let Ok(mut active) = lock.lock() {
            *active = active.saturating_sub(1);
            available.notify_one();
        }
    }
}
