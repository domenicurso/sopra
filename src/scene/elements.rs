use ratatui::style::Style;
use unicode_segmentation::UnicodeSegmentation;

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
        let mut flow = super::text_flow::TextFlow::new(
            crate::input::CursorPosition {
                column: self.column,
                row: self.row,
            },
            canvas.buffer.area.width,
        );
        for span in &self.spans {
            for grapheme in span.text.graphemes(true) {
                if let Some(position) = flow.advance(grapheme) {
                    canvas.text(TextRun {
                        position: (position.column, position.row),
                        text: grapheme,
                        style: span.style,
                        max_width: canvas.buffer.area.width.saturating_sub(position.column),
                    });
                }
            }
        }
    }
}

pub(super) struct CommandElement {
    pub(super) column: u16,
    pub(super) row: u16,
    pub(super) buffer: String,
    pub(super) spans: Vec<SyntaxSpan>,
    pub(super) selection: Option<std::ops::Range<usize>>,
    pub(super) secondary: Vec<PromptSpan>,
}

impl Element for CommandElement {
    fn paint(&self, canvas: &mut Canvas) {
        let mut flow = super::text_flow::TextFlow::new(
            crate::input::CursorPosition {
                column: self.column,
                row: self.row,
            },
            canvas.buffer.area.width,
        );
        for (byte, grapheme) in self.buffer.grapheme_indices(true) {
            if grapheme == "\n" {
                flow.advance(grapheme);
                let start = flow.position;
                PromptElement {
                    column: start.column,
                    row: start.row,
                    spans: self.secondary.clone(),
                }
                .paint(canvas);
                flow.prompt(&self.secondary);
                continue;
            }
            let Some(position) = flow.advance(grapheme) else {
                continue;
            };
            let mut style = syntax::style_at(&self.spans, byte);
            if self
                .selection
                .as_ref()
                .is_some_and(|selection| selection.contains(&byte))
            {
                style = style.add_modifier(ratatui::style::Modifier::REVERSED);
            }
            canvas.text(TextRun {
                position: (position.column, position.row),
                text: grapheme,
                style,
                max_width: canvas.buffer.area.width.saturating_sub(position.column),
            });
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
