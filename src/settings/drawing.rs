use ratatui::{buffer::Buffer, layout::Rect, style::Style};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub(super) fn draw_text(
    buffer: &mut Buffer,
    column: u16,
    row: u16,
    text: &str,
    max_width: u16,
    style: Style,
) -> u16 {
    let mut offset: u16 = 0;
    for grapheme in text.graphemes(true) {
        let width = UnicodeWidthStr::width(grapheme) as u16;
        if width == 0 || offset.saturating_add(width) > max_width {
            break;
        }
        buffer.set_string(column + offset, row, grapheme, style);
        offset += width;
    }
    offset
}

pub(super) fn fill(buffer: &mut Buffer, area: Rect, style: Style) {
    for row in area.top()..area.bottom() {
        for column in area.left()..area.right() {
            if let Some(cell) = buffer.cell_mut((column, row)) {
                cell.set_symbol(" ").set_style(style);
            }
        }
    }
}
