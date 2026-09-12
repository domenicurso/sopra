use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph as RatatuiParagraph, Widget},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{
    component::Component,
    geometry::{Constraints, Size, line_width},
    style::StyleToken,
};

pub struct InputLine {
    prefix: Line<'static>,
    text: String,
    cursor: usize,
    text_style: Style,
    cursor_style: Style,
}

impl InputLine {
    pub fn new(prefix: impl Into<String>, text: impl Into<String>, cursor: usize) -> Self {
        Self {
            prefix: Line::from(prefix.into()),
            text: text.into(),
            cursor,
            text_style: Style::default(),
            cursor_style: StyleToken::Cursor.style(),
        }
    }

    pub const fn text_style(mut self, style: Style) -> Self {
        self.text_style = style;
        self
    }

    pub const fn cursor_style(mut self, style: Style) -> Self {
        self.cursor_style = style;
        self
    }
}

impl Component for InputLine {
    fn measure(&self, constraints: Constraints) -> Size {
        let text_width = self.text.width() as u16;
        Size::new(
            (line_width(&self.prefix) as u16)
                .saturating_add(text_width)
                .min(constraints.max_width),
            1.min(constraints.max_height),
        )
    }

    fn render(&self, area: Rect, buffer: &mut Buffer) {
        let graphemes = self.text.graphemes(true).collect::<Vec<_>>();
        let cursor = self.cursor.min(graphemes.len());
        let mut spans = self.prefix.spans.clone();
        spans.push(Span::styled(graphemes[..cursor].concat(), self.text_style));
        if cursor < graphemes.len() {
            spans.push(Span::styled(
                graphemes[cursor].to_string(),
                self.cursor_style,
            ));
            spans.push(Span::styled(
                graphemes[cursor + 1..].concat(),
                self.text_style,
            ));
        } else {
            spans.push(Span::styled(" ", self.cursor_style));
        }
        RatatuiParagraph::new(Line::from(spans)).render(area, buffer);
    }
}
