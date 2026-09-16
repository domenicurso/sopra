use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
};

use super::canvas::Canvas;

const BLOCK_UNITS: usize = 8;
const LOWER_BLOCKS: [&str; BLOCK_UNITS + 1] = [" ", "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

pub(super) fn paint(canvas: &mut Canvas, area: Rect, start: usize, visible: usize, total: usize) {
    if area.width == 0 || area.height == 0 || total == 0 || visible == 0 {
        return;
    }
    let track_units = usize::from(area.height) * BLOCK_UNITS;
    let thumb_units = track_units
        .saturating_mul(visible)
        .div_ceil(total)
        .max(1)
        .min(track_units);
    let max_start = total.saturating_sub(visible);
    let thumb_top = start
        .min(max_start)
        .saturating_mul(track_units.saturating_sub(thumb_units))
        .checked_div(max_start)
        .unwrap_or(0);
    for row in 0..usize::from(area.height) {
        let row_start = row * BLOCK_UNITS;
        let row_end = row_start + BLOCK_UNITS;
        let overlap_start = thumb_top.max(row_start);
        let overlap_end = thumb_top.saturating_add(thumb_units).min(row_end);
        let overlap = overlap_end.saturating_sub(overlap_start);
        let (symbol, style) = scrollbar_cell(overlap, overlap_start == row_start);
        canvas.put(area.left(), area.top() + row as u16, symbol, style);
    }
}

fn scrollbar_cell(overlap: usize, enters: bool) -> (&'static str, Style) {
    match (overlap, enters) {
        (0, _) => (
            "█",
            Style::default()
                .fg(Color::DarkGray)
                .remove_modifier(Modifier::DIM),
        ),
        (BLOCK_UNITS, _) => (
            " ",
            Style::default()
                .bg(Color::White)
                .remove_modifier(Modifier::DIM),
        ),
        (amount, true) => (
            LOWER_BLOCKS[BLOCK_UNITS - amount],
            Style::default()
                .fg(Color::DarkGray)
                .bg(Color::White)
                .remove_modifier(Modifier::DIM),
        ),
        (amount, false) => (
            LOWER_BLOCKS[amount],
            Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray)
                .remove_modifier(Modifier::DIM),
        ),
    }
}
