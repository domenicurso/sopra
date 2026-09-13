mod app_suggestions;
mod buffer;
mod events;
mod geometry;
mod host;
mod state;
mod suggestion;

pub use app_suggestions::SUGGESTION_VIEWPORT_ROWS;
pub use buffer::EditorBuffer;
pub use events::{EditorAction, EditorEvent};
pub use geometry::{ScreenPoint, TerminalSize};
pub use host::{HostLine, HostSnapshot};
pub use state::{AppState, CursorState, EditorMode};
pub use suggestion::{CompletionProvider, Suggestion};

#[cfg(test)]
mod tests {
    mod editor;
    mod selection;
}
