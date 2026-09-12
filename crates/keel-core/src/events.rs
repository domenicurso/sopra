use crate::{AppState, HostSnapshot, TerminalSize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorEvent {
    Begin(HostSnapshot),
    Redisplay(HostSnapshot),
    Resize(TerminalSize),
    Accept,
    Cancel,
    Finish,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorAction {
    Redraw,
    Accepted(String),
    Cleared,
    Noop,
}

impl AppState {
    pub fn apply(&mut self, event: EditorEvent) -> EditorAction {
        match event {
            EditorEvent::Begin(snapshot) | EditorEvent::Redisplay(snapshot) => {
                self.observe(&snapshot);
                EditorAction::Redraw
            }
            EditorEvent::Resize(size) => {
                let changed = self.terminal != size;
                self.terminal = size;
                self.dirty |= changed;
                if changed {
                    EditorAction::Redraw
                } else {
                    EditorAction::Noop
                }
            }
            EditorEvent::Accept => EditorAction::Accepted(self.buffer.text().to_string()),
            EditorEvent::Cancel | EditorEvent::Finish => {
                if self.clear() {
                    EditorAction::Cleared
                } else {
                    EditorAction::Noop
                }
            }
        }
    }
}
