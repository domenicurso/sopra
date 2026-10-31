use super::{compiler, watch::Watch};
use sopra::{Action, Context, Event, Refresh, protocol::Response};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant},
};

pub(super) struct Input {
    pub event: Event,
    pub reply: mpsc::SyncSender<Action>,
}

struct Session {
    watch: Watch,
    context: Option<Context>,
    next: Option<Instant>,
    checked: Instant,
}

pub(super) fn run(
    source: PathBuf,
    state: (Arc<Mutex<Context>>, Arc<Mutex<Response>>),
    events: mpsc::Receiver<Input>,
) {
    let mut session = Session {
        watch: Watch::default(),
        context: None,
        next: None,
        checked: Instant::now(),
    };
    loop {
        session.refresh_source(&source, &state.1);
        let wait = session
            .next
            .map_or(Duration::from_millis(250), |next: Instant| {
                next.saturating_duration_since(Instant::now())
                    .min(Duration::from_millis(250))
            });
        let input = match events.recv_timeout(wait) {
            Ok(input) => Some(input),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        };
        let latest = state
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        session.poll(latest, input, &state.1);
    }
}

impl Session {
    fn refresh_source(&mut self, source: &std::path::Path, output: &Mutex<Response>) {
        if Instant::now() < self.checked {
            return;
        }
        match self.watch.refresh(source) {
            Ok(true) => self.context = None,
            Ok(false) => {}
            Err(error) => {
                publish_error(output, &error);
                self.watch.process = None;
            }
        }
        self.checked = Instant::now() + Duration::from_millis(250);
    }

    fn poll(&mut self, latest: Context, input: Option<Input>, output: &Mutex<Response>) {
        let due = self.context.as_ref() != Some(&latest)
            || input.is_some()
            || self.next.is_some_and(|next| Instant::now() >= next);
        if !due {
            return;
        }
        let Some(process) = self.watch.process.as_mut() else {
            self.next = None;
            return;
        };
        match process.request(&latest, input.as_ref().map(|input| input.event)) {
            Ok(mut response) => {
                self.next = match response.refresh {
                    Refresh::OnContext => None,
                    Refresh::Every(interval) => Instant::now().checked_add(interval),
                };
                self.context = Some(latest);
                let action = std::mem::take(&mut response.action);
                *output.lock().unwrap_or_else(|error| error.into_inner()) = response;
                if let Some(input) = input {
                    let _ = input.reply.send(action);
                }
            }
            Err(error) => {
                publish_error(output, &error);
                self.watch.process = None;
                self.next = None;
            }
        }
    }
}

fn publish_error(output: &Mutex<Response>, error: &std::io::Error) {
    let message = if compiler::record_error(error).is_ok() {
        "widget error"
    } else {
        "widget error: log unavailable"
    };
    *output.lock().unwrap_or_else(|error| error.into_inner()) = Response {
        text: message.into(),
        ..Response::default()
    };
}
