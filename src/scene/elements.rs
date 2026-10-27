use ratatui::style::Style;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::{
    Element,
    canvas::{Canvas, TextRun},
};
use crate::{
    prompt::PromptSpan,
    syntax::{self, SyntaxSpan},
};

pub(super) struct PromptElement {
    pub(super) column: u16,
    pub(super) row: u16,
    pub(super) spans: Vec<PromptSpan>,
}

impl Element for PromptElement {
    fn paint(&self, canvas: &mut Canvas) {
        let mut row = self.row;
        let mut column = self.column;
        for span in &self.spans {
            for (index, text) in span.text.split('\n').enumerate() {
                if index > 0 {
                    row = row.saturating_add(1);
                    column = 0;
                }
                if row >= canvas.buffer.area.height {
                    return;
                }
                canvas.text(TextRun {
                    position: (column, row),
                    text,
                    style: span.style,
                    max_width: canvas.buffer.area.width.saturating_sub(column),
                });
                column = column.saturating_add(unicode_width::UnicodeWidthStr::width(text) as u16);
            }
        }
    }
}

pub(super) struct CommandElement {
    pub(super) column: u16,
    pub(super) row: u16,
    pub(super) buffer: String,
    pub(super) spans: Vec<SyntaxSpan>,
    pub(super) max_width: u16,
}

impl Element for CommandElement {
    fn paint(&self, canvas: &mut Canvas) {
        let width = canvas
            .buffer
            .area
            .width
            .saturating_sub(self.column)
            .min(self.max_width);
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
    pub(super) symbol: String,
    pub(super) width: u16,
    pub(super) style: Style,
}

impl Element for CursorElement {
    fn paint(&self, canvas: &mut Canvas) {
        canvas.text(TextRun {
            position: (self.column, self.row),
            text: &self.symbol,
            style: self.style,
            max_width: self.width.max(1),
        });
    }
}
