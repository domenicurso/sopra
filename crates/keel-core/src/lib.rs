use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub const SUGGESTION_VIEWPORT_ROWS: usize = 12;

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
    replacement: String,
}

impl Suggestion {
    pub fn new(label: impl Into<String>, detail: impl Into<String>) -> Self {
        let label = label.into();
        Self {
            replacement: label.clone(),
            label,
            detail: detail.into(),
        }
    }

    pub fn with_replacement(
        label: impl Into<String>,
        detail: impl Into<String>,
        replacement: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            detail: detail.into(),
            replacement: replacement.into(),
        }
    }

    pub fn replacement(&self) -> &str {
        &self.replacement
    }
}

pub trait CompletionProvider {
    fn complete(&self, line: &str, cursor: usize, cwd: &str) -> Vec<Suggestion>;
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
    pub selected_suggestion: Option<usize>,
    suggestion_scroll: usize,
    pub overlay_dismissed: bool,
    suppressed_overlay_text: Option<String>,
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
            suggestions: Vec::new(),
            selected_suggestion: None,
            suggestion_scroll: 0,
            overlay_dismissed: false,
            suppressed_overlay_text: None,
            mode: EditorMode::ObservingZle,
            dirty: true,
            redisplay_generation: snapshot.redisplay_generation,
        }
    }

    pub fn observe(&mut self, snapshot: &HostSnapshot) -> bool {
        let snapshot = snapshot.sanitized();
        let next_buffer = snapshot.line.buffer();
        let line_changed = self.buffer.text() != next_buffer.text();
        let suppressed = self.suppressed_overlay_text.as_deref() == Some(next_buffer.text());
        let changed = self.buffer != next_buffer
            || self.cursor.screen != snapshot.cursor
            || self.terminal != snapshot.terminal
            || self.cwd != snapshot.cwd
            || self.keymap != snapshot.keymap
            || self.last_status != snapshot.last_status
            || self.redisplay_generation != snapshot.redisplay_generation;

        self.buffer = next_buffer;
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

    pub fn selected(&self) -> Option<&Suggestion> {
        self.selected_suggestion
            .and_then(|selected| self.suggestions.get(selected))
    }

    pub fn suggestions_visible(&self) -> bool {
        !self.overlay_dismissed && !self.suggestions.is_empty()
    }

    pub fn selected_replacement(&self) -> Option<&str> {
        self.selected().map(Suggestion::replacement)
    }

    pub fn suggestion_viewport_start(&self) -> usize {
        self.suggestion_scroll
    }

    pub fn completion_query(&self) -> String {
        let cursor = self.buffer.cursor_byte_offset();
        self.buffer.text()[..cursor]
            .rsplit(char::is_whitespace)
            .next()
            .unwrap_or_default()
            .to_string()
    }

    pub fn completion_token_width(&self) -> u16 {
        self.completion_query().width().min(u16::MAX as usize) as u16
    }

    pub fn set_suggestions(&mut self, suggestions: Vec<Suggestion>) -> bool {
        let had_selection = self.selected_suggestion.is_some();
        let changed = self.suggestions != suggestions;
        self.suggestions = suggestions;
        if changed {
            self.selected_suggestion = had_selection
                .then_some(0)
                .filter(|_| !self.suggestions.is_empty());
            self.suggestion_scroll = 0;
        } else if self
            .selected_suggestion
            .is_some_and(|selected| selected >= self.suggestions.len())
        {
            self.selected_suggestion = None;
            self.suggestion_scroll = 0;
        }
        self.dirty |= changed;
        changed
    }

    pub fn move_selection(&mut self, delta: isize) -> bool {
        if !self.suggestions_visible() {
            return false;
        }

        let len = self.suggestions.len() as isize;
        let next = match self.selected_suggestion {
            Some(selected) => (selected as isize + delta).rem_euclid(len) as usize,
            None if delta < 0 => len.saturating_sub(1) as usize,
            None => 0,
        };
        self.selected_suggestion = Some(next);
        self.update_suggestion_scroll(next);
        self.dirty = true;
        true
    }

    fn update_suggestion_scroll(&mut self, selected: usize) {
        let length = self.suggestions.len();
        let max_start = length.saturating_sub(SUGGESTION_VIEWPORT_ROWS);
        if max_start == 0 {
            self.suggestion_scroll = 0;
            return;
        }

        // Keep two rows available in the direction of travel. The selected
        // item moves the viewport when it reaches the eleventh visible row,
        // then moves it back when it reaches the second visible row.
        if selected >= self.suggestion_scroll + SUGGESTION_VIEWPORT_ROWS - 2 {
            let desired_start = selected.saturating_sub(SUGGESTION_VIEWPORT_ROWS - 3);
            self.suggestion_scroll = self
                .suggestion_scroll
                .max(desired_start)
                .min(max_start);
        } else if selected <= self.suggestion_scroll + 1 {
            self.suggestion_scroll = self
                .suggestion_scroll
                .min(selected.saturating_sub(2));
        }
    }

    pub fn dismiss_overlay(&mut self) -> bool {
        let changed = self.suggestions_visible();
        self.overlay_dismissed = true;
        self.suppressed_overlay_text = Some(self.buffer.text().to_string());
        self.dirty |= changed;
        changed
    }

    pub fn suppress_overlay_for_text(&mut self, text: impl Into<String>) {
        self.overlay_dismissed = true;
        self.suppressed_overlay_text = Some(text.into());
        self.dirty = true;
    }

    pub fn clear(&mut self) -> bool {
        let changed = !self.buffer.text().is_empty() || self.buffer.cursor() != 0;
        self.buffer = EditorBuffer::new("", 0);
        self.suggestions.clear();
        self.selected_suggestion = None;
        self.suggestion_scroll = 0;
        self.overlay_dismissed = false;
        self.suppressed_overlay_text = None;
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
    fn selection_wraps_without_owning_the_host_line() {
        let snapshot = HostSnapshot {
            line: HostLine::new("git", 3),
            ..HostSnapshot::default()
        };
        let mut app = AppState::from_snapshot(&snapshot);
        app.set_suggestions(vec![
            Suggestion::new("run git", ""),
            Suggestion::new("inspect git", ""),
            Suggestion::new("search git", ""),
        ]);
        assert!(app.move_selection(-1));
        assert_eq!(
            app.selected().map(|suggestion| suggestion.label.as_str()),
            Some("search git")
        );
        assert_eq!(app.buffer.text(), "git");
    }

    #[test]
    fn dismissed_overlay_stays_hidden_until_the_line_changes() {
        let snapshot = HostSnapshot {
            line: HostLine::new("git", 3),
            ..HostSnapshot::default()
        };
        let mut app = AppState::from_snapshot(&snapshot);
        app.set_suggestions(vec![Suggestion::new("run git", "")]);
        assert!(app.suggestions_visible());
        assert!(app.dismiss_overlay());
        assert!(!app.suggestions_visible());

        let same_line = HostSnapshot {
            redisplay_generation: 2,
            ..snapshot.clone()
        };
        app.observe(&same_line);
        assert!(!app.suggestions_visible());

        let changed_line = HostSnapshot {
            line: HostLine::new("git ", 4),
            ..same_line
        };
        app.observe(&changed_line);
        app.set_suggestions(vec![Suggestion::new("run git", "")]);
        assert!(app.suggestions_visible());
    }

    #[test]
    fn moving_selection_does_not_reopen_a_dismissed_overlay() {
        let snapshot = HostSnapshot {
            line: HostLine::new("git", 3),
            ..HostSnapshot::default()
        };
        let mut app = AppState::from_snapshot(&snapshot);
        app.set_suggestions(vec![
            Suggestion::new("run git", ""),
            Suggestion::new("inspect git", ""),
        ]);
        app.dismiss_overlay();
        assert!(!app.move_selection(1));
        assert!(!app.suggestions_visible());
    }

    #[test]
    fn selection_is_empty_until_down_moves_into_the_list() {
        let snapshot = HostSnapshot {
            line: HostLine::new("git", 3),
            ..HostSnapshot::default()
        };
        let mut app = AppState::from_snapshot(&snapshot);
        app.set_suggestions(vec![
            Suggestion::new("git", ""),
            Suggestion::new("git-receive-pack", ""),
        ]);

        assert_eq!(app.selected_suggestion, None);
        assert!(app.move_selection(1));
        assert_eq!(app.selected_suggestion, Some(0));
    }

    #[test]
    fn selection_scrolls_at_the_eleventh_and_second_visible_rows() {
        let snapshot = HostSnapshot {
            line: HostLine::new("command ", 8),
            ..HostSnapshot::default()
        };
        let mut app = AppState::from_snapshot(&snapshot);
        app.set_suggestions(
            (0..20)
                .map(|index| Suggestion::new(format!("item-{index}"), ""))
                .collect(),
        );

        for _ in 0..10 {
            assert!(app.move_selection(1));
        }
        assert_eq!(app.selected_suggestion, Some(9));
        assert_eq!(app.suggestion_viewport_start(), 0);

        assert!(app.move_selection(1));
        assert_eq!(app.selected_suggestion, Some(10));
        assert_eq!(app.suggestion_viewport_start(), 1);

        for _ in 0..9 {
            assert!(app.move_selection(1));
        }
        assert_eq!(app.selected_suggestion, Some(19));
        assert_eq!(app.suggestion_viewport_start(), 8);

        for _ in 0..10 {
            assert!(app.move_selection(-1));
        }
        assert_eq!(app.selected_suggestion, Some(9));
        assert_eq!(app.suggestion_viewport_start(), 7);

        assert!(app.move_selection(-1));
        assert_eq!(app.selected_suggestion, Some(8));
        assert_eq!(app.suggestion_viewport_start(), 6);
    }
}
