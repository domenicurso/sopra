use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub columns: u16,
    pub rows: u16,
}

impl TerminalSize {
    pub const fn new(columns: u16, rows: u16) -> Self {
        Self {
            columns: if columns == 0 { 1 } else { columns },
            rows: if rows == 0 { 1 } else { rows },
        }
    }
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self::new(80, 24)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScreenPoint {
    pub column: u16,
    pub row: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorBuffer {
    text: String,
    cursor: usize,
}

impl EditorBuffer {
    pub fn new(text: impl Into<String>, cursor: usize) -> Self {
        let text = text.into();
        let cursor = cursor.min(text.graphemes(true).count());
        Self { text, cursor }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn grapheme_count(&self) -> usize {
        self.text.graphemes(true).count()
    }

    pub fn cursor_byte_offset(&self) -> usize {
        self.text
            .grapheme_indices(true)
            .nth(self.cursor)
            .map_or(self.text.len(), |(offset, _)| offset)
    }

    pub fn cursor_width(&self) -> usize {
        self.text
            .graphemes(true)
            .take(self.cursor)
            .collect::<String>()
            .width()
    }

    pub fn insert(&mut self, value: &str) -> bool {
        if value.is_empty() {
            return false;
        }

        let offset = self.cursor_byte_offset();
        self.text.insert_str(offset, value);
        self.cursor += value.graphemes(true).count();
        true
    }

    pub fn backspace(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }

        let graphemes = self.text.graphemes(true).collect::<Vec<_>>();
        self.text = graphemes[..self.cursor - 1]
            .iter()
            .chain(graphemes[self.cursor..].iter())
            .copied()
            .collect();
        self.cursor -= 1;
        true
    }

    pub fn delete(&mut self) -> bool {
        if self.cursor >= self.grapheme_count() {
            return false;
        }

        let graphemes = self.text.graphemes(true).collect::<Vec<_>>();
        self.text = graphemes[..self.cursor]
            .iter()
            .chain(graphemes[self.cursor + 1..].iter())
            .copied()
            .collect();
        true
    }

    pub fn move_left(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        true
    }

    pub fn move_right(&mut self) -> bool {
        if self.cursor >= self.grapheme_count() {
            return false;
        }
        self.cursor += 1;
        true
    }

    pub fn move_home(&mut self) -> bool {
        let changed = self.cursor != 0;
        self.cursor = 0;
        changed
    }

    pub fn move_end(&mut self) -> bool {
        let end = self.grapheme_count();
        let changed = self.cursor != end;
        self.cursor = end;
        changed
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostLine {
    pub text: String,
    pub cursor: usize,
}

impl HostLine {
    pub fn new(text: impl Into<String>, cursor: usize) -> Self {
        let buffer = EditorBuffer::new(text, cursor);
        Self {
            text: buffer.text().to_string(),
            cursor: buffer.cursor(),
        }
    }

    pub fn from_codepoint_cursor(text: impl Into<String>, cursor: usize) -> Self {
        let text = text.into();
        let codepoint_cursor = cursor.min(text.chars().count());
        let grapheme_cursor = text
            .chars()
            .take(codepoint_cursor)
            .collect::<String>()
            .graphemes(true)
            .count();
        Self::new(text, grapheme_cursor)
    }

    pub fn buffer(&self) -> EditorBuffer {
        EditorBuffer::new(self.text.clone(), self.cursor)
    }
}

impl Default for HostLine {
    fn default() -> Self {
        Self::new("", 0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostSnapshot {
    pub line: HostLine,
    pub terminal: TerminalSize,
    pub cursor: ScreenPoint,
    pub cwd: String,
    pub keymap: String,
    pub last_status: i32,
    pub redisplay_generation: u64,
}

impl Default for HostSnapshot {
    fn default() -> Self {
        Self {
            line: HostLine::default(),
            terminal: TerminalSize::default(),
            cursor: ScreenPoint::default(),
            cwd: "~".to_string(),
            keymap: "main".to_string(),
            last_status: 0,
            redisplay_generation: 0,
        }
    }
}

impl HostSnapshot {
    pub fn sanitized(&self) -> Self {
        let terminal = TerminalSize::new(self.terminal.columns, self.terminal.rows);
        Self {
            line: HostLine::new(self.line.text.clone(), self.line.cursor),
            terminal,
            cursor: ScreenPoint {
                column: self.cursor.column.min(terminal.columns.saturating_sub(1)),
                row: self.cursor.row.min(terminal.rows.saturating_sub(1)),
            },
            cwd: if self.cwd.is_empty() {
                "~".to_string()
            } else {
                self.cwd.clone()
            },
            keymap: if self.keymap.is_empty() {
                "main".to_string()
            } else {
                self.keymap.clone()
            },
            last_status: self.last_status,
            redisplay_generation: self.redisplay_generation,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub label: String,
    pub detail: String,
}

impl Suggestion {
    pub fn new(label: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            detail: detail.into(),
        }
    }
}

pub fn suggestions_for(input: &str) -> Vec<Suggestion> {
    let input = input.trim_end();
    if input.is_empty() {
        return Vec::new();
    }

    if input.ends_with("grep --matches") {
        return vec![
            Suggestion::new("--files-with-matches", "grep option"),
            Suggestion::new("--files-without-match", "grep option"),
        ];
    }

    vec![
        Suggestion::new(format!("run {input}"), "execute this command"),
        Suggestion::new(format!("inspect {input}"), "inspect the command"),
        Suggestion::new(format!("search {input}"), "search related history"),
    ]
}

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
    pub selected_suggestion: usize,
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
        Self {
            buffer: snapshot.line.buffer(),
            cursor: CursorState {
                screen: snapshot.cursor,
                visible: true,
            },
            terminal: snapshot.terminal,
            cwd: snapshot.cwd,
            keymap: snapshot.keymap,
            last_status: snapshot.last_status,
            suggestions: suggestions_for(&snapshot.line.text),
            selected_suggestion: 0,
            mode: EditorMode::ObservingZle,
            dirty: true,
            redisplay_generation: snapshot.redisplay_generation,
        }
    }

    pub fn observe(&mut self, snapshot: &HostSnapshot) -> bool {
        let snapshot = snapshot.sanitized();
        let next_buffer = snapshot.line.buffer();
        let next_suggestions = suggestions_for(&snapshot.line.text);
        let changed = self.buffer != next_buffer
            || self.cursor.screen != snapshot.cursor
            || self.terminal != snapshot.terminal
            || self.cwd != snapshot.cwd
            || self.keymap != snapshot.keymap
            || self.last_status != snapshot.last_status
            || self.suggestions != next_suggestions
            || self.redisplay_generation != snapshot.redisplay_generation;

        self.buffer = next_buffer;
        self.cursor.screen = snapshot.cursor;
        self.terminal = snapshot.terminal;
        self.cwd = snapshot.cwd;
        self.keymap = snapshot.keymap;
        self.last_status = snapshot.last_status;
        self.suggestions = next_suggestions;
        if self.selected_suggestion >= self.suggestions.len() {
            self.selected_suggestion = 0;
        }
        self.redisplay_generation = snapshot.redisplay_generation;
        self.dirty = changed;
        changed
    }

    pub fn selected(&self) -> Option<&Suggestion> {
        self.suggestions.get(self.selected_suggestion)
    }

    pub fn move_selection(&mut self, delta: isize) -> bool {
        if self.suggestions.is_empty() {
            return false;
        }

        let len = self.suggestions.len() as isize;
        let next = (self.selected_suggestion as isize + delta).rem_euclid(len) as usize;
        let changed = next != self.selected_suggestion;
        self.selected_suggestion = next;
        self.dirty |= changed;
        changed
    }

    pub fn clear(&mut self) -> bool {
        let changed = !self.buffer.text().is_empty() || self.buffer.cursor() != 0;
        self.buffer = EditorBuffer::new("", 0);
        self.suggestions.clear();
        self.selected_suggestion = 0;
        self.dirty = true;
        changed
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_are_grapheme_aware() {
        let mut buffer = EditorBuffer::new("a🙂é", 3);
        assert_eq!(buffer.grapheme_count(), 3);
        assert_eq!(buffer.cursor_byte_offset(), buffer.text().len());
        buffer.backspace();
        assert_eq!(buffer.text(), "a🙂");
        assert_eq!(buffer.cursor(), 2);
        buffer.move_left();
        buffer.delete();
        assert_eq!(buffer.text(), "a");
        buffer.insert("界");
        assert_eq!(buffer.text(), "a界");
        assert_eq!(buffer.cursor_width(), 3);
    }

    #[test]
    fn host_cursor_is_clamped_to_graphemes_and_terminal() {
        let snapshot = HostSnapshot {
            line: HostLine::new("🙂", 99),
            terminal: TerminalSize::new(10, 4),
            cursor: ScreenPoint {
                column: 99,
                row: 99,
            },
            ..HostSnapshot::default()
        }
        .sanitized();
        assert_eq!(snapshot.line.cursor, 1);
        assert_eq!(snapshot.cursor, ScreenPoint { column: 9, row: 3 });
    }

    #[test]
    fn codepoint_cursor_is_translated_to_a_grapheme_cursor() {
        let line = HostLine::from_codepoint_cursor("a é", 3);
        assert_eq!(line.text, "a é");
        assert_eq!(line.cursor, 3);
    }

    #[test]
    fn suggestions_are_empty_for_an_empty_line() {
        assert!(suggestions_for("   ").is_empty());
    }

    #[test]
    fn grep_suggestions_match_the_completion_poc() {
        let suggestions = suggestions_for("grep --matches");
        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0].label, "--files-with-matches");
        assert_eq!(suggestions[1].label, "--files-without-match");
    }

    #[test]
    fn selection_wraps_without_owning_the_host_line() {
        let snapshot = HostSnapshot {
            line: HostLine::new("git", 3),
            ..HostSnapshot::default()
        };
        let mut app = AppState::from_snapshot(&snapshot);
        assert!(app.move_selection(-1));
        assert_eq!(
            app.selected().map(|suggestion| suggestion.label.as_str()),
            Some("search git")
        );
        assert_eq!(app.buffer.text(), "git");
    }
}
