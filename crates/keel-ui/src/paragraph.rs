use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, Paragraph as RatatuiParagraph, Widget, Wrap},
};

use crate::{
    component::Component,
    geometry::{Constraints, Size, block_dimensions, line_width, wrapped_line_height},
};

pub struct Paragraph {
    lines: Vec<Line<'static>>,
    style: Style,
    block: Option<Block<'static>>,
    wrap: bool,
}

impl Paragraph {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            lines: text
                .into()
                .split('\n')
                .map(|line| Line::from(line.to_string()))
                .collect(),
            style: Style::default(),
            block: None,
            wrap: true,
        }
    }

    pub fn from_lines(lines: Vec<Line<'static>>) -> Self {
        Self {
            lines,
            style: Style::default(),
            block: None,
            wrap: true,
        }
    }

    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn block(mut self, block: Block<'static>) -> Self {
        self.block = Some(block);
        self
    }

    pub const fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }
}

impl Component for Paragraph {
    fn measure(&self, constraints: Constraints) -> Size {
        let (horizontal, vertical) = self
            .block
            .as_ref()
            .map_or((0, 0), |block| block_dimensions(block, constraints));
        let content_width = constraints.max_width.saturating_sub(horizontal);
        let content_height = if self.wrap {
            self.lines
                .iter()
                .map(|line| wrapped_line_height(line, content_width))
                .sum::<usize>()
        } else {
            self.lines.len().max(1)
        } as u16;
        let content_width_used = self.lines.iter().map(line_width).max().unwrap_or(0) as u16;
        Size::new(
            content_width_used
                .saturating_add(horizontal)
                .min(constraints.max_width),
            content_height
                .saturating_add(vertical)
                .min(constraints.max_height),
        )
    }

    fn render(&self, area: Rect, buffer: &mut Buffer) {
        let mut paragraph = RatatuiParagraph::new(self.lines.clone()).style(self.style);
        if self.wrap {
            paragraph = paragraph.wrap(Wrap { trim: false });
        }
        if let Some(block) = self.block.clone() {
            paragraph = paragraph.block(block);
        }
        paragraph.render(area, buffer);
    }
}
