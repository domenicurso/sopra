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
        let mut flow = crate::text::flow::TextFlow::new(
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

pub(super) struct LayoutPrompt(pub(super) std::rc::Rc<crate::text::layout::Layout>);

impl Element for LayoutPrompt {
    fn paint(&self, canvas: &mut Canvas) {
        for glyph in &self.0.prompt {
            canvas.text(TextRun {
                position: (glyph.position.column, glyph.position.row),
                text: &glyph.text,
                style: glyph.style,
                max_width: canvas
                    .buffer
                    .area
                    .width
                    .saturating_sub(glyph.position.column),
            });
        }
    }
}

pub(super) struct CommandElement {
    pub(super) layout: std::rc::Rc<crate::text::layout::Layout>,
    pub(super) spans: Vec<SyntaxSpan>,
    pub(super) selection: Option<std::ops::Range<usize>>,
}

impl Element for CommandElement {
    fn paint(&self, canvas: &mut Canvas) {
        for glyph in &self.layout.command {
            let position = glyph.position;
            let mut style = glyph
                .byte
                .map_or(glyph.style, |byte| syntax::style_at(&self.spans, byte));
            if self
                .selection
                .as_ref()
                .is_some_and(|selection| glyph.byte.is_some_and(|byte| selection.contains(&byte)))
            {
                style = style.add_modifier(ratatui::style::Modifier::REVERSED);
            }
            canvas.text(TextRun {
                position: (position.column, position.row),
                text: &glyph.text,
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
