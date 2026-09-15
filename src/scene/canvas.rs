use ratatui::{buffer::Buffer, layout::Rect, style::Style};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub(super) struct Canvas {
    pub(super) buffer: Buffer,
}

pub(super) struct TextRun<'a> {
    pub(super) position: (u16, u16),
    pub(super) text: &'a str,
    pub(super) style: Style,
    pub(super) max_width: u16,
}

impl Canvas {
    pub(super) fn new(area: Rect) -> Self {
        Self {
            buffer: Buffer::empty(area),
        }
    }

    pub(super) fn into_buffer(self) -> Buffer {
        self.buffer
    }

    pub(super) fn put(&mut self, column: u16, row: u16, symbol: &str, style: Style) {
        if let Some(cell) = self.buffer.cell_mut((column, row)) {
            cell.set_symbol(symbol).set_style(style);
        }
    }

    pub(super) fn text(&mut self, run: TextRun<'_>) {
        let (mut column, row) = run.position;
        if self.buffer.cell((column, row)).is_none() {
            return;
        }
        let text = run.text;
        let style = run.style;
        let max_width = run.max_width;
        let right = column.saturating_add(max_width);
        for grapheme in text.graphemes(true) {
            if grapheme.chars().any(char::is_control) {
                continue;
            }
            let width = grapheme.width() as u16;
            if width == 0 {
                continue;
            }
            if column.saturating_add(width) > right {
                break;
            }
            self.buffer.set_string(column, row, grapheme, style);
            column = column.saturating_add(width);
        }
    }

    pub(super) fn border(&mut self, area: Rect, style: Style) {
        if area.width < 2 || area.height < 2 {
            return;
        }
        let right = area.right().saturating_sub(1);
        let bottom = area.bottom().saturating_sub(1);
        for column in area.left()..=right {
            self.put(column, area.top(), "─", style);
            self.put(column, bottom, "─", style);
        }
        for row in area.top()..=bottom {
            self.put(area.left(), row, "│", style);
            self.put(right, row, "│", style);
        }
        self.put(area.left(), area.top(), "╭", style);
        self.put(right, area.top(), "╮", style);
        self.put(area.left(), bottom, "╰", style);
        self.put(right, bottom, "╯", style);
    }
}
