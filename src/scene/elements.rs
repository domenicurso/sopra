use ratatui::style::Style;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::{
    Element,
    canvas::{Canvas, TextRun},
};
use crate::{
    prompt::{self, PromptSpan},
    syntax::{self, SyntaxSpan},
};

pub(super) struct PromptElement {
    pub(super) column: u16,
    pub(super) row: u16,
    pub(super) spans: Vec<PromptSpan>,
}

impl Element for PromptElement {
    fn paint(&self, canvas: &mut Canvas) {
        let mut offset = 0_u16;
        for span in &self.spans {
            canvas.text(TextRun {
                position: (self.column.saturating_add(offset), self.row),
                text: &span.text,
                style: span.style,
                max_width: canvas
                    .buffer
                    .area
                    .width
                    .saturating_sub(self.column + offset),
            });
            offset = offset.saturating_add(prompt::width(std::slice::from_ref(span)) as u16);
        }
    }
}

impl PromptElement {
    pub(super) fn styled(text: &str, style: Style) -> Self {
        Self {
            column: 0,
            row: 0,
            spans: vec![PromptSpan {
                text: text.to_string(),
                style,
            }],
        }
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
