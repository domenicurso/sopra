use unicode_segmentation::UnicodeSegmentation;

use crate::{EditorBuffer, ScreenPoint, TerminalSize};

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
