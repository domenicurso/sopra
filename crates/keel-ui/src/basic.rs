use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph as RatatuiParagraph, Widget, Wrap},
};

use crate::{
    component::Component,
    geometry::{Constraints, Size, line_width, wrapped_line_height},
};

pub struct Spacer {
    size: Size,
    style: Style,
}

impl Spacer {
    pub const fn new(width: u16, height: u16) -> Self {
        Self {
            size: Size::new(width, height),
            style: Style::new(),
        }
    }

    pub const fn height(height: u16) -> Self {
        Self::new(0, height)
    }

    pub const fn width(width: u16) -> Self {
        Self::new(width, 0)
    }

    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Component for Spacer {
    fn measure(&self, constraints: Constraints) -> Size {
        Size::new(
            self.size.width.min(constraints.max_width),
            self.size.height.min(constraints.max_height),
        )
    }

    fn render(&self, area: Rect, buffer: &mut Buffer) {
        if self.style != Style::default() {
            buffer.set_style(area, self.style);
        }
    }
}

pub struct Text {
    lines: Vec<Line<'static>>,
    wrap: bool,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            lines: text
                .into()
                .split('\n')
                .map(|line| Line::from(line.to_string()))
                .collect(),
            wrap: false,
        }
    }

    pub fn styled(text: impl Into<String>, style: Style) -> Self {
        Self {
            lines: vec![Line::from(Span::styled(text.into(), style))],
            wrap: false,
        }
    }

    pub fn lines(lines: Vec<Line<'static>>) -> Self {
        Self { lines, wrap: false }
    }

    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }
}

impl Component for Text {
    fn measure(&self, constraints: Constraints) -> Size {
        let width = self.lines.iter().map(line_width).max().unwrap_or(0);
        let height = if self.wrap {
            self.lines
                .iter()
                .map(|line| wrapped_line_height(line, constraints.max_width))
                .sum::<usize>()
        } else {
            self.lines.len().max(1)
        };
        Size::new(
            (width as u16).min(constraints.max_width),
            (height as u16).min(constraints.max_height),
        )
    }

    fn render(&self, area: Rect, buffer: &mut Buffer) {
        let mut paragraph = RatatuiParagraph::new(self.lines.clone());
        if self.wrap {
            paragraph = paragraph.wrap(Wrap { trim: false });
        }
        paragraph.render(area, buffer);
    }
}
