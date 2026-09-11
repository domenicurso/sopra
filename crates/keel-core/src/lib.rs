use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostSnapshot {
    pub buffer: String,
    pub cursor: usize,
    pub columns: u16,
    pub rows: u16,
    pub keymap: String,
    pub last_status: i32,
    pub cwd: String,
}

impl Default for HostSnapshot {
    fn default() -> Self {
        Self {
            buffer: String::new(),
            cursor: 0,
            columns: 80,
            rows: 24,
            keymap: "main".to_string(),
            last_status: 0,
            cwd: "~".to_string(),
        }
    }
}

impl HostSnapshot {
    pub fn sanitized(&self) -> Self {
        let cursor = self.cursor.min(self.buffer.chars().count());
        Self {
            buffer: self.buffer.clone(),
            cursor,
            columns: self.columns.max(1),
            rows: self.rows.max(1),
            keymap: if self.keymap.is_empty() {
                "main".to_string()
            } else {
                self.keymap.clone()
            },
            last_status: self.last_status,
            cwd: if self.cwd.is_empty() {
                "~".to_string()
            } else {
                self.cwd.clone()
            },
        }
    }

    pub fn character_count(&self) -> usize {
        self.buffer.chars().count()
    }

    pub fn grapheme_count(&self) -> usize {
        self.buffer.graphemes(true).count()
    }

    pub fn cursor_width(&self) -> usize {
        self.buffer
            .graphemes(true)
            .take(self.cursor.min(self.grapheme_count()))
            .collect::<String>()
            .width()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    Insert,
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

    pub fn insert(&mut self, value: &str) -> bool {
        if value.is_empty() {
            return false;
        }

        let graphemes = self.text.graphemes(true).collect::<Vec<_>>();
        let left = graphemes[..self.cursor].concat();
        let right = graphemes[self.cursor..].concat();
        self.text = format!("{left}{value}{right}");
        self.cursor += value.graphemes(true).count();
        true
    }

    pub fn backspace(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }

        let mut graphemes = self.text.graphemes(true).collect::<Vec<_>>();
        graphemes.remove(self.cursor - 1);
        self.text = graphemes.concat();
        self.cursor -= 1;
        true
    }

    pub fn delete(&mut self) -> bool {
        let count = self.grapheme_count();
        if self.cursor >= count {
            return false;
        }

        let mut graphemes = self.text.graphemes(true).collect::<Vec<_>>();
        graphemes.remove(self.cursor);
        self.text = graphemes.concat();
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
pub enum EditorEvent {
    SyncHost(HostSnapshot),
    Insert(String),
    Backspace,
    Delete,
    MoveLeft,
    MoveRight,
    MoveHome,
    MoveEnd,
    Resize { columns: u16, rows: u16 },
    Accept,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorAction {
    Changed,
    Moved,
    Resized,
    Accepted(String),
    Cancelled,
    Noop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorState {
    pub grapheme: usize,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppState {
    pub buffer: EditorBuffer,
    pub cursor: CursorState,
    pub mode: EditorMode,
    pub columns: u16,
    pub rows: u16,
    pub dirty: bool,
}

impl AppState {
    pub fn from_host(snapshot: HostSnapshot) -> Self {
        let snapshot = snapshot.sanitized();
        let buffer = EditorBuffer::new(snapshot.buffer, snapshot.cursor);
        Self {
            cursor: CursorState {
                grapheme: buffer.cursor(),
                visible: true,
            },
            buffer,
            mode: EditorMode::Insert,
            columns: snapshot.columns,
            rows: snapshot.rows,
            dirty: true,
        }
    }

    pub fn apply(&mut self, event: EditorEvent) -> EditorAction {
        let action = match event {
            EditorEvent::SyncHost(snapshot) => self.sync_host(snapshot),
            EditorEvent::Insert(value) => {
                let changed = self.buffer.insert(&value);
                self.changed(changed)
            }
            EditorEvent::Backspace => {
                let changed = self.buffer.backspace();
                self.changed(changed)
            }
            EditorEvent::Delete => {
                let changed = self.buffer.delete();
                self.changed(changed)
            }
            EditorEvent::MoveLeft => {
                let moved = self.buffer.move_left();
                self.moved(moved)
            }
            EditorEvent::MoveRight => {
                let moved = self.buffer.move_right();
                self.moved(moved)
            }
            EditorEvent::MoveHome => {
                let moved = self.buffer.move_home();
                self.moved(moved)
            }
            EditorEvent::MoveEnd => {
                let moved = self.buffer.move_end();
                self.moved(moved)
            }
            EditorEvent::Resize { columns, rows } => {
                let changed = self.columns != columns.max(1) || self.rows != rows.max(1);
                self.columns = columns.max(1);
                self.rows = rows.max(1);
                if changed {
                    self.dirty = true;
                    EditorAction::Resized
                } else {
                    EditorAction::Noop
                }
            }
            EditorEvent::Accept => EditorAction::Accepted(self.buffer.text().to_string()),
            EditorEvent::Cancel => {
                let changed = !self.buffer.text().is_empty() || self.buffer.cursor() != 0;
                self.buffer = EditorBuffer::new(String::new(), 0);
                self.cursor.grapheme = 0;
                self.dirty = true;
                if changed {
                    EditorAction::Cancelled
                } else {
                    EditorAction::Noop
                }
            }
        };

        self.cursor.grapheme = self.buffer.cursor();
        action
    }

    fn sync_host(&mut self, snapshot: HostSnapshot) -> EditorAction {
        let snapshot = snapshot.sanitized();
        let next_buffer = EditorBuffer::new(snapshot.buffer, snapshot.cursor);
        let changed = self.buffer != next_buffer
            || self.columns != snapshot.columns
            || self.rows != snapshot.rows;
        self.buffer = next_buffer;
        self.columns = snapshot.columns;
        self.rows = snapshot.rows;
        self.dirty = changed;
        if changed {
            EditorAction::Changed
        } else {
            EditorAction::Noop
        }
    }

    fn changed(&mut self, changed: bool) -> EditorAction {
        if changed {
            self.dirty = true;
            EditorAction::Changed
        } else {
            EditorAction::Noop
        }
    }

    fn moved(&mut self, moved: bool) -> EditorAction {
        if moved {
            self.dirty = true;
            EditorAction::Moved
        } else {
            EditorAction::Noop
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AppState, EditorAction, EditorBuffer, EditorEvent, HostSnapshot};

    #[test]
    fn sanitizes_cursor_and_terminal_dimensions() {
        let snapshot = HostSnapshot {
            buffer: "hello".to_string(),
            cursor: 99,
            columns: 0,
            rows: 0,
            keymap: String::new(),
            last_status: 0,
            cwd: String::new(),
        }
        .sanitized();

        assert_eq!(snapshot.cursor, 5);
        assert_eq!(snapshot.columns, 1);
        assert_eq!(snapshot.rows, 1);
        assert_eq!(snapshot.keymap, "main");
    }

    #[test]
    fn measures_cursor_by_terminal_cells() {
        let snapshot = HostSnapshot {
            buffer: "a界b".to_string(),
            cursor: 2,
            ..HostSnapshot::default()
        };

        assert_eq!(snapshot.cursor_width(), 3);
    }

    #[test]
    fn edits_by_grapheme_cluster() {
        let mut buffer = EditorBuffer::new("a界b", 2);

        assert!(buffer.backspace());
        assert_eq!(buffer.text(), "ab");
        assert_eq!(buffer.cursor(), 1);
        assert!(buffer.insert("界"));
        assert_eq!(buffer.text(), "a界b");
        assert_eq!(buffer.cursor(), 2);
    }

    #[test]
    fn cancel_clears_in_place_without_accepting() {
        let mut state = AppState::from_host(HostSnapshot {
            buffer: "echo hi".to_string(),
            cursor: 7,
            ..HostSnapshot::default()
        });

        assert_eq!(state.apply(EditorEvent::Cancel), EditorAction::Cancelled);
        assert_eq!(state.buffer.text(), "");
        assert_eq!(state.cursor.grapheme, 0);
    }
}
