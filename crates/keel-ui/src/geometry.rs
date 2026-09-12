use ratatui::{layout::Rect, text::Line, widgets::Block};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Size {
    pub width: u16,
    pub height: u16,
}

impl Size {
    pub const fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Constraints {
    pub max_width: u16,
    pub max_height: u16,
}

impl Constraints {
    pub const fn new(max_width: u16, max_height: u16) -> Self {
        Self {
            max_width,
            max_height,
        }
    }
}

pub(crate) fn line_width(line: &Line<'_>) -> usize {
    line.spans
        .iter()
        .map(|span| span.content.as_ref().width())
        .sum()
}

pub(crate) fn wrapped_line_height(line: &Line<'_>, width: u16) -> usize {
    if width == 0 {
        return 1;
    }
    line_width(line).max(1).div_ceil(width as usize)
}

pub(crate) fn block_dimensions(block: &Block<'_>, constraints: Constraints) -> (u16, u16) {
    let inner = block.inner(Rect::new(
        0,
        0,
        constraints.max_width,
        constraints.max_height,
    ));
    (
        constraints.max_width.saturating_sub(inner.width),
        constraints.max_height.saturating_sub(inner.height),
    )
}
