use crate::{EditorBuffer, HostSnapshot, ScreenPoint, Suggestion, TerminalSize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    ObservingZle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorState {
    pub screen: ScreenPoint,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppState {
    pub buffer: EditorBuffer,
    pub cursor: CursorState,
    pub terminal: TerminalSize,
    pub cwd: String,
    pub keymap: String,
    pub last_status: i32,
    pub suggestions: Vec<Suggestion>,
    pub(crate) completion_source: Vec<Suggestion>,
    pub(crate) completion_source_key: Option<String>,
    pub(crate) current_completion_context_key: String,
    pub(crate) ranked_query: Option<String>,
    pub selected_suggestion: Option<usize>,
    pub(crate) suggestion_scroll: usize,
    pub overlay_dismissed: bool,
    pub(crate) suppressed_overlay_text: Option<String>,
    pub mode: EditorMode,
    pub dirty: bool,
    pub redisplay_generation: u64,
}

impl Default for AppState {
    fn default() -> Self {
        Self::from_snapshot(&HostSnapshot::default())
    }
}

impl AppState {
    pub fn from_snapshot(snapshot: &HostSnapshot) -> Self {
        let snapshot = snapshot.sanitized();
        let mut state = Self {
            buffer: snapshot.line.buffer(),
            cursor: CursorState {
                screen: snapshot.cursor,
                visible: true,
            },
            terminal: snapshot.terminal,
            cwd: snapshot.cwd,
            keymap: snapshot.keymap,
            last_status: snapshot.last_status,
            suggestions: Vec::new(),
            completion_source: Vec::new(),
            completion_source_key: None,
            current_completion_context_key: String::new(),
            ranked_query: None,
            selected_suggestion: None,
            suggestion_scroll: 0,
            overlay_dismissed: false,
            suppressed_overlay_text: None,
            mode: EditorMode::ObservingZle,
            dirty: true,
            redisplay_generation: snapshot.redisplay_generation,
        };
        state.current_completion_context_key = state.completion_context_key();
        state
    }

    pub fn observe(&mut self, snapshot: &HostSnapshot) -> bool {
        let snapshot = snapshot.sanitized();
        let next_buffer = snapshot.line.buffer();
        let buffer_changed = self.buffer != next_buffer;
        let line_changed = self.buffer.text() != next_buffer.text();
        let suppressed = self.suppressed_overlay_text.as_deref() == Some(next_buffer.text());
        let changed = buffer_changed
            || self.cursor.screen != snapshot.cursor
            || self.terminal != snapshot.terminal
            || self.cwd != snapshot.cwd
            || self.keymap != snapshot.keymap
            || self.last_status != snapshot.last_status
            || self.redisplay_generation != snapshot.redisplay_generation;

        self.buffer = next_buffer;
        if buffer_changed {
            self.current_completion_context_key = self.completion_context_key();
        }
        self.cursor.screen = snapshot.cursor;
        self.terminal = snapshot.terminal;
        self.cwd = snapshot.cwd;
        self.keymap = snapshot.keymap;
        self.last_status = snapshot.last_status;
        if suppressed {
            self.overlay_dismissed = true;
        } else if line_changed {
            self.overlay_dismissed = false;
            self.suppressed_overlay_text = None;
            self.suggestions.clear();
            self.ranked_query = None;
            self.selected_suggestion = None;
            self.suggestion_scroll = 0;
        }
        if self
            .selected_suggestion
            .is_some_and(|selected| selected >= self.suggestions.len())
        {
            self.selected_suggestion = None;
        }
        self.redisplay_generation = snapshot.redisplay_generation;
        self.dirty = changed;
        changed
    }
}
