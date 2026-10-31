use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};

use super::{
    compiler,
    process::{Output, Process},
};

struct Event {
    key: String,
    reply: mpsc::SyncSender<Option<PathBuf>>,
}

pub(super) struct Runtime {
    context: Arc<Mutex<String>>,
    output: Arc<Mutex<Output>>,
    events: mpsc::SyncSender<Event>,
}

impl Runtime {
    pub(super) fn start(source: PathBuf, context: String) -> Self {
        let context = Arc::new(Mutex::new(context));
        let output = Arc::new(Mutex::new(Output::default()));
        let (events, receiver) = mpsc::sync_channel(8);
        let state = (Arc::clone(&context), Arc::clone(&output));
        thread::spawn(move || run(source, state, receiver));
        Self {
            context,
            output,
            events,
        }
    }

    pub(super) fn update(&self, context: String) {
        *self
            .context
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = context;
    }

    pub(super) fn display(&self) -> (String, bool, bool) {
        let output = self
            .output
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        (output.text.clone(), output.active, output.interactive)
    }

    pub(super) fn key(&self, key: String) -> Option<PathBuf> {
        let (reply, response) = mpsc::sync_channel(1);
        if self.events.try_send(Event { key, reply }).is_err() {
            return None;
        }
        response
            .recv_timeout(Duration::from_millis(150))
            .ok()
            .flatten()
    }
}

fn run(
    source: PathBuf,
    state: (Arc<Mutex<String>>, Arc<Mutex<Output>>),
    events: mpsc::Receiver<Event>,
) {
    let mut previous = None;
    let mut process = None;
    loop {
        refresh(&source, &mut previous, &mut process, &state.1);
        let context = state
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        let event = match events.recv_timeout(Duration::from_millis(250)) {
            Ok(event) => Some(event),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        };
        let Some(child) = process.as_mut() else {
            continue;
        };
        let result = child.request(
            &context,
            event.as_ref().map_or("", |event| event.key.as_str()),
        );
        match result {
            Ok(mut output) => {
                if let Some(event) = event {
                    let _ = event.reply.send(output.directory.take());
                }
                *state.1.lock().unwrap_or_else(|error| error.into_inner()) = output;
            }
            Err(error) => {
                publish_error(&state.1, &error);
                process = None;
            }
        }
    }
}

fn refresh(
    source: &std::path::Path,
    previous: &mut Option<Vec<u8>>,
    process: &mut Option<Process>,
    output: &Mutex<Output>,
) {
    let bytes = match fs::read(source) {
        Ok(bytes) => bytes,
        Err(error) => {
            publish_error(output, &error);
            *process = None;
            return;
        }
    };
    if previous.as_ref() == Some(&bytes) {
        return;
    }
    let result = compiler::build(source, &bytes).and_then(|binary| Process::start(&binary));
    *previous = Some(bytes);
    match result {
        Ok(child) => {
            *process = Some(child);
        }
        Err(error) => {
            publish_error(output, &error);
            *process = None;
        }
    }
}

fn publish_error(output: &Mutex<Output>, error: &std::io::Error) {
    *output.lock().unwrap_or_else(|error| error.into_inner()) = Output {
        text: if compiler::record_error(error).is_ok() {
            "widget error".into()
        } else {
            "widget error: log unavailable".into()
        },
        ..Output::default()
    };
}
