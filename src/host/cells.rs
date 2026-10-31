use ratatui::{
    buffer::Buffer,
    style::{Color, Modifier, Style},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
pub(super) fn text(buffer: &mut Buffer, position: (u16, u16), text: &str, style: Style) {
    let (mut column, row) = position;
    for grapheme in text.graphemes(true) {
        let display = if grapheme.chars().any(char::is_control) {
            "�"
        } else {
            grapheme
        };
        let width = display.width() as u16;
        if column.saturating_add(width) > buffer.area.width {
            break;
        }
        if let Some(cell) = buffer.cell_mut((column, row)) {
            cell.set_symbol(display).set_style(style);
        }
        column = column.saturating_add(width);
    }
}
pub(super) fn highlight(
    buffer: &mut Buffer,
    row: u16,
    source: &super::rows::Row,
    range: std::ops::Range<usize>,
) {
    let Some(prefix) = source.text.get(..range.start) else {
        return;
    };
    let Some(selected) = source.text.get(range) else {
        return;
    };
    let start = 2 + width(prefix);
    let end = start + width(selected);
    for column in start..end.min(buffer.area.width) {
        if let Some(cell) = buffer.cell_mut((column, row)) {
            cell.set_style(Style::default().add_modifier(Modifier::REVERSED));
        }
    }
}
pub(super) fn copy_screen(
    buffer: &mut Buffer,
    screen: &vt100::Screen,
    rows: std::ops::Range<u16>,
    target: u16,
) {
    for (offset, row) in rows.enumerate() {
        for column in 0..buffer.area.width {
            let Some(cell) = screen
                .cell(row, column)
                .filter(|cell| !cell.is_wide_continuation())
            else {
                continue;
            };
            let Some(target) = buffer.cell_mut((column, target + offset as u16)) else {
                continue;
            };
            let contents = cell.contents();
            target
                .set_symbol(if contents.is_empty() { " " } else { contents })
                .set_style(cell_style(cell));
        }
    }
}
fn cell_style(cell: &vt100::Cell) -> Style {
    let mut style = Style::default()
        .fg(color(cell.fgcolor()))
        .bg(color(cell.bgcolor()));
    for (on, modifier) in [
        (cell.bold(), Modifier::BOLD),
        (cell.dim(), Modifier::DIM),
        (cell.italic(), Modifier::ITALIC),
        (cell.underline(), Modifier::UNDERLINED),
        (cell.inverse(), Modifier::REVERSED),
    ] {
        if on {
            style = style.add_modifier(modifier);
        }
    }
    style
}
fn color(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(index) => Color::Indexed(index),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

pub(super) fn width(text: &str) -> u16 {
    text.graphemes(true)
        .map(|text| {
            if text.chars().any(char::is_control) {
                1
            } else {
                text.width() as u16
            }
        })
        .sum()
}
