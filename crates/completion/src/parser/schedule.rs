use super::{
    control::Control,
    discover::{HelpRequest, HelpResponse},
};
use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex, mpsc::Sender},
    time::{Duration, Instant},
};

const QUEUED_LIMIT: usize = 64;

pub(crate) struct Dispatcher {
    shared: Arc<(Mutex<State>, Condvar)>,
}

#[derive(Default)]
struct State {
    urgent: VecDeque<HelpRequest>,
    background: VecDeque<HelpRequest>,
    active: Option<(String, Control)>,
    closed: bool,
}

impl Dispatcher {
    pub(crate) fn spawn(responses: Sender<HelpResponse>) -> Option<Self> {
        let shared = Arc::new((Mutex::new(State::default()), Condvar::new()));
        let dispatcher = Self {
            shared: shared.clone(),
        };
        std::thread::Builder::new()
            .name("completion-discovery".into())
            .spawn(move || run(shared, responses))
            .ok()?;
        Some(dispatcher)
    }

    pub(crate) fn send(&self, request: HelpRequest, background: bool) -> bool {
        let (lock, ready) = &*self.shared;
        let Ok(mut state) = lock.lock() else {
            return false;
        };
        if !background
            && let Some((id, control)) = &state.active
            && *id != request.id()
        {
            control.cancel();
        }
        if state.urgent.len() + state.background.len() >= QUEUED_LIMIT {
            return false;
        }
        if background {
            state.background.push_back(request);
        } else {
            for queued in &state.urgent {
                queued.control.cancel();
            }
            state.urgent.push_back(request);
        }
        ready.notify_one();
        true
    }

    pub(crate) fn promote(&self, id: &str) {
        let (lock, ready) = &*self.shared;
        if let Ok(mut state) = lock.lock() {
            if let Some(index) = state
                .background
                .iter()
                .position(|request| request.id() == id)
                && let Some(request) = state.background.remove(index)
            {
                state.urgent.push_front(request);
            }
            if let Some((active, control)) = &state.active
                && active != id
            {
                control.cancel();
            }
            ready.notify_one();
        }
    }

    pub(crate) fn cancel(&self) {
        let (lock, _) = &*self.shared;
        if let Ok(state) = lock.lock() {
            if let Some((_, control)) = &state.active {
                control.cancel();
            }
            for request in state.urgent.iter().chain(&state.background) {
                request.control.cancel();
            }
        }
    }
}

impl Drop for Dispatcher {
    fn drop(&mut self) {
        self.cancel();
        let (lock, ready) = &*self.shared;
        if let Ok(mut state) = lock.lock() {
            state.closed = true;
            ready.notify_one();
        }
    }
}

fn run(shared: Arc<(Mutex<State>, Condvar)>, responses: Sender<HelpResponse>) {
    loop {
        let Some(request) = next(&shared) else {
            return;
        };
        let started = Instant::now();
        let graph = request
            .environment
            .with(|| super::discover::resolve_chunk(&request));
        let cancelled = request.control.cancelled();
        let response = HelpResponse {
            id: request.id(),
            key: request.key,
            path: request.args,
            request: request.request,
            graph,
            elapsed: started.elapsed(),
            cancelled,
        };
        if responses.send(response).is_err() {
            return;
        }
        let (lock, _) = &*shared;
        if let Ok(mut state) = lock.lock() {
            state.active = None;
        }
        request.control.cancel();
    }
}

fn next(shared: &(Mutex<State>, Condvar)) -> Option<HelpRequest> {
    let (lock, ready) = shared;
    let mut state = lock.lock().ok()?;
    loop {
        if state.closed {
            return None;
        }
        if let Some(mut request) = state
            .urgent
            .pop_front()
            .or_else(|| state.background.pop_front())
        {
            if !request.control.stopped() {
                request.control = Control::new(Duration::from_secs(2));
            }
            state.active = Some((request.id(), request.control.clone()));
            return Some(request);
        }
        state = ready.wait(state).ok()?;
    }
}

#[cfg(test)]
#[path = "schedule_tests.rs"]
mod tests;
