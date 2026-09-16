use ratatui::style::Style;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::{
    Element,
    canvas::{Canvas, TextRun},
};
use crate::syntax::{self, SyntaxSpan};

pub(super) struct PromptElement {
    pub(super) column: u16,
    pub(super) row: u16,
    pub(super) text: String,
    pub(super) style: Style,
}

impl Element for PromptElement {
    fn paint(&self, canvas: &mut Canvas) {
        canvas.text(TextRun {
            position: (self.column, self.row),
            text: &self.text,
            style: self.style,
            max_width: canvas.buffer.area.width.saturating_sub(self.column),
        });
    }
}

pub(super) struct CommandElement {
    pub(super) column: u16,
    pub(super) row: u16,
    pub(super) buffer: String,
    pub(super) spans: Vec<SyntaxSpan>,
}

impl Element for CommandElement {
    fn paint(&self, canvas: &mut Canvas) {
        let width = canvas.buffer.area.width.saturating_sub(self.column);
        let mut offset = 0_u16;
        for (byte, grapheme) in self.buffer.grapheme_indices(true) {
            if grapheme.chars().any(char::is_control) {
                continue;
            }
            let grapheme_width = grapheme.width() as u16;
            if grapheme_width == 0 {
                continue;
            }
            canvas.text(TextRun {
                position: (self.column.saturating_add(offset), self.row),
                text: grapheme,
                style: syntax::style_at(&self.spans, byte),
                max_width: width.saturating_sub(offset),
            });
            offset = offset.saturating_add(grapheme_width);
        }
    }
}

pub(super) struct CursorElement {
    pub(super) column: u16,
    pub(super) row: u16,
    pub(super) width: u16,
    pub(super) style: Style,
}

impl Element for CursorElement {
    fn paint(&self, canvas: &mut Canvas) {
        for offset in 0..self.width.max(1) {
            canvas.put(
                self.column.saturating_add(offset),
                self.row,
                " ",
                self.style,
            );
        }
    }
}

pub(super) fn display_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}
