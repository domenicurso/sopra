use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{input::CursorPosition, prompt::PromptSpan};

#[derive(Clone, Copy)]
pub(crate) struct TextFlow {
    pub(crate) position: CursorPosition,
    columns: u16,
}

impl TextFlow {
    pub(crate) fn new(position: CursorPosition, columns: u16) -> Self {
        Self {
            position,
            columns: columns.max(1),
        }
    }

    pub(crate) fn advance(&mut self, text: &str) -> Option<CursorPosition> {
        if text == "\n" {
            self.position.row = self.position.row.saturating_add(1);
            self.position.column = 0;
            return None;
        }
        if text.chars().any(char::is_control) {
            return None;
        }
        let width = text.width().min(self.columns as usize) as u16;
        if width == 0 {
            return None;
        }
        if self.position.column.saturating_add(width) > self.columns {
            self.position.row = self.position.row.saturating_add(1);
            self.position.column = 0;
        }
        let position = self.position;
        self.position.column = self.position.column.saturating_add(width);
        Some(position)
    }

    pub(crate) fn cursor(&self) -> CursorPosition {
        if self.position.column >= self.columns {
            CursorPosition {
                row: self.position.row.saturating_add(1),
                column: 0,
            }
        } else {
            self.position
        }
    }

    pub(crate) fn text(&mut self, text: &str) {
        for grapheme in text.graphemes(true) {
            self.advance(grapheme);
        }
    }

    pub(crate) fn prompt(&mut self, spans: &[PromptSpan]) {
        for span in spans {
            self.text(&span.text);
        }
    }
}
