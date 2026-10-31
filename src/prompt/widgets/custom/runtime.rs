use super::session::{Input, run};
use sopra::{Action, Context, Event, protocol::Response};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};

pub(super) struct Runtime {
    context: Arc<Mutex<Context>>,
    output: Arc<Mutex<Response>>,
    events: mpsc::SyncSender<Input>,
}

impl Runtime {
    pub(super) fn start(source: PathBuf, context: Context) -> Self {
        let context = Arc::new(Mutex::new(context));
        let output = Arc::new(Mutex::new(Response::default()));
        let (events, receiver) = mpsc::sync_channel(8);
        let state = (Arc::clone(&context), Arc::clone(&output));
        thread::spawn(move || run(source, state, receiver));
        Self {
            context,
            output,
            events,
        }
    }
    pub(super) fn update(&self, context: Context) {
        *self
            .context
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = context;
    }
    pub(super) fn display(&self) -> Response {
        self.output
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }
    pub(super) fn key(&self, event: Event) -> Action {
        let (reply, response) = mpsc::sync_channel(1);
        if self.events.try_send(Input { event, reply }).is_err() {
            return Action::Ignored;
        }
        response
            .recv_timeout(Duration::from_millis(150))
            .unwrap_or(Action::Consumed)
    }
}
