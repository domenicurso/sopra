use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
pub(crate) struct Control {
    cancelled: Arc<AtomicBool>,
    deadline: Instant,
}

impl Control {
    pub(crate) fn new(budget: Duration) -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            deadline: Instant::now() + budget,
        }
    }
    pub(crate) fn child(&self, budget: Duration) -> Self {
        Self {
            cancelled: self.cancelled.clone(),
            deadline: self.deadline.min(Instant::now() + budget),
        }
    }
    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
    pub(crate) fn cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
    pub(crate) fn stopped(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed) || Instant::now() >= self.deadline
    }
}
