use super::session::{Input, run};
use sopra::{Action, Context, Event, protocol::Response};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};

pub(super) enum Status {
    Loading,
    Ready,
    Failed(String),
}

pub(super) struct State {
    pub(super) context: Mutex<Context>,
    pub(super) output: Mutex<Response>,
    pub(super) status: Mutex<Status>,
}

pub(in crate::prompt::widgets) struct Runtime {
    state: Arc<State>,
    events: mpsc::SyncSender<Input>,
}

impl Runtime {
    pub(super) fn start(source: PathBuf, context: Context) -> Self {
        let state = Arc::new(State {
            context: Mutex::new(context),
            output: Mutex::new(Response::default()),
            status: Mutex::new(Status::Loading),
        });
        let (events, receiver) = mpsc::sync_channel(8);
        let shared = state.clone();
        thread::spawn(move || run(source, shared, receiver));
        Self { state, events }
    }
    pub(super) fn update(&self, context: Context) {
        *self
            .state
            .context
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = context;
    }
    pub(super) fn display(&self) -> Response {
        self.state
            .output
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }
    pub(in crate::prompt::widgets) fn status(&self) -> String {
        match &*self
            .state
            .status
            .lock()
            .unwrap_or_else(|error| error.into_inner())
        {
            Status::Loading => "loading".into(),
            Status::Ready => "loaded".into(),
            Status::Failed(error) => format!("error: {error}"),
        }
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
