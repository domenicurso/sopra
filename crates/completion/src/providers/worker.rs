use super::{cache::Cache, execute, query::Query};
use crate::{CompletionItem, parser::control::Control};
use std::{
    sync::{
        Arc, Condvar, Mutex,
        mpsc::{self, Receiver, Sender},
    },
    time::{Duration, Instant},
};

pub(crate) struct Worker {
    shared: Arc<(Mutex<State>, Condvar)>,
    thread: Option<std::thread::JoinHandle<()>>,
    pub(crate) responses: Receiver<Response>,
}
#[derive(Default)]
struct State {
    queued: Option<Query>,
    active: Option<Control>,
    closed: bool,
}
pub(crate) struct Response {
    pub(crate) query: Query,
    pub(crate) items: Vec<CompletionItem>,
    pub(crate) matched: bool,
    pub(crate) elapsed: Duration,
}

impl Worker {
    pub(crate) fn spawn() -> Option<Self> {
        let shared = Arc::new((Mutex::new(State::default()), Condvar::new()));
        let (sender, responses) = mpsc::channel();
        let thread_shared = shared.clone();
        let thread = std::thread::Builder::new()
            .name("completion-values".into())
            .spawn(move || run(thread_shared, sender))
            .ok()?;
        Some(Self {
            shared,
            responses,
            thread: Some(thread),
        })
    }
    pub(crate) fn submit(&self, query: Query) {
        let (lock, ready) = &*self.shared;
        if let Ok(mut state) = lock.lock() {
            if let Some(control) = &state.active {
                control.cancel();
            }
            state.queued = Some(query);
            ready.notify_one();
        }
    }
    pub(crate) fn cancel(&self) {
        let (lock, _) = &*self.shared;
        if let Ok(mut state) = lock.lock() {
            if let Some(control) = &state.active {
                control.cancel();
            }
            state.queued = None;
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel();
        let (lock, ready) = &*self.shared;
        if let Ok(mut state) = lock.lock() {
            state.closed = true;
            ready.notify_one();
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run(shared: Arc<(Mutex<State>, Condvar)>, responses: Sender<Response>) {
    let mut cache = Cache::new();
    while let Some((query, control)) = next(&shared) {
        let started = Instant::now();
        let result = query
            .environment
            .with(|| execute::resolve(&query, &control, &mut cache));
        if control.stopped() {
            continue;
        }
        let values = result.ok().flatten();
        let matched = values.is_some();
        let items = super::items(&query, values.unwrap_or_default());
        if responses
            .send(Response {
                query,
                items,
                matched,
                elapsed: started.elapsed(),
            })
            .is_err()
        {
            return;
        }
    }
}

fn next(shared: &(Mutex<State>, Condvar)) -> Option<(Query, Control)> {
    let (lock, ready) = shared;
    let mut state = lock.lock().ok()?;
    loop {
        if state.closed {
            return None;
        }
        if let Some(query) = state.queued.take() {
            let control = Control::new(Duration::from_millis(500));
            state.active = Some(control.clone());
            return Some((query, control));
        }
        state = ready.wait(state).ok()?;
    }
}
