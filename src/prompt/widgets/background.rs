use sopra::{Action, Context, Event, Refresh, Text, Widget};
use std::{
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

#[derive(Default)]
struct Output {
    text: Text,
    interactive: bool,
    active: bool,
}

enum Request {
    Context(Context),
    Event(Event),
}

pub(super) struct Background {
    requests: mpsc::Sender<Request>,
    output: Arc<Mutex<Output>>,
    actions: mpsc::Receiver<(Event, Action)>,
    context: Option<Context>,
}

impl Background {
    pub(super) fn new(widget: Box<dyn Widget + Send>) -> Self {
        let (requests, receiver) = mpsc::channel();
        let (actions, action_receiver) = mpsc::channel();
        let output = Arc::new(Mutex::new(Output::default()));
        let published = output.clone();
        thread::spawn(move || run(widget, receiver, published, actions));
        Self {
            requests,
            output,
            actions: action_receiver,
            context: None,
        }
    }
    pub(super) fn poll_action(&self) -> Option<(Event, Action)> {
        self.actions.try_recv().ok()
    }
    pub(super) fn update(&mut self, context: Context) {
        if self.context.as_ref() != Some(&context) {
            self.context = Some(context.clone());
            let _ = self.requests.send(Request::Context(context));
        }
    }
    pub(super) fn display(&self) -> Text {
        self.output
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .text
            .clone()
    }
    pub(super) fn interactive(&self) -> bool {
        self.output
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .interactive
    }
    pub(super) fn active(&self) -> bool {
        self.output.lock().unwrap_or_else(|e| e.into_inner()).active
    }
    pub(super) fn handle(&self, event: Event) {
        let _ = self.requests.send(Request::Event(event));
    }
}

fn run(
    mut widget: Box<dyn Widget + Send>,
    requests: mpsc::Receiver<Request>,
    output: Arc<Mutex<Output>>,
    actions: mpsc::Sender<(Event, Action)>,
) {
    let mut context = None;
    let mut next = None;
    loop {
        let wait = next.map_or(Duration::from_secs(3600), |deadline: Instant| {
            deadline.saturating_duration_since(Instant::now())
        });
        let mut response = None;
        match requests.recv_timeout(wait) {
            Ok(Request::Context(latest)) => context = Some(latest),
            Ok(Request::Event(event)) => {
                if let Some(context) = &context {
                    let action = widget.handle(event, context);
                    response = Some((event, action));
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        let Some(context) = &context else { continue };
        let text = widget.render(context);
        *output.lock().unwrap_or_else(|e| e.into_inner()) = Output {
            text,
            interactive: widget.interactive(),
            active: widget.active(),
        };
        if let Some(response) = response {
            let _ = actions.send(response);
        }
        next = match widget.refresh() {
            Refresh::OnContext => None,
            Refresh::Every(interval) => {
                Instant::now().checked_add(interval.max(Duration::from_millis(1)))
            }
        };
    }
}

#[cfg(test)]
mod tests;
