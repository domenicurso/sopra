use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
};

const BLOCK_UNITS: usize = 8;
const LOWER_BLOCKS: [&str; BLOCK_UNITS + 1] = [" ", "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

pub(crate) fn render_scrollbar(
    area: Rect,
    item_count: usize,
    viewport_start: usize,
    visible_count: usize,
    buffer: &mut Buffer,
) {
    if area.width == 0 || area.height == 0 || item_count == 0 || visible_count == 0 {
        return;
    }

    let track_height = area.height as usize;
    let track_units = track_height.saturating_mul(BLOCK_UNITS);
    let thumb_units = track_units
        .saturating_mul(visible_count)
        .div_ceil(item_count)
        .max(1)
        .min(track_units);
    let max_thumb_top = track_units.saturating_sub(thumb_units);
    let max_viewport_start = item_count.saturating_sub(visible_count);
    let thumb_top = viewport_start
        .min(max_viewport_start)
        .saturating_mul(max_thumb_top)
        .checked_div(max_viewport_start)
        .unwrap_or(0);
    for row in 0..track_height {
        let Some(cell) = buffer.cell_mut((area.x, area.y + row as u16)) else {
            continue;
        };
        let row_start = row * BLOCK_UNITS;
        let row_end = row_start + BLOCK_UNITS;
        let thumb_end = thumb_top.saturating_add(thumb_units);
        let overlap_start = thumb_top.max(row_start);
        let overlap_end = thumb_end.min(row_end);
        let overlap = overlap_end.saturating_sub(overlap_start);

        match overlap {
            0 => {
                // Use the palette's dark gray foreground so the track remains
                // visibly faint even in terminals that ignore DIM.
                cell.set_symbol("█");
                cell.set_style(
                    Style::default()
                        .fg(Color::DarkGray)
                        .remove_modifier(Modifier::DIM),
                );
            }
            BLOCK_UNITS => {
                cell.set_symbol(" ");
                cell.set_style(
                    Style::default()
                        .bg(Color::White)
                        .remove_modifier(Modifier::DIM),
                );
            }
            _ if overlap_start == row_start => {
                // A partial thumb entering from above uses a white background
                // and a dark lower block for the remaining track.
                let uncovered = row_end - overlap_end;
                cell.set_symbol(LOWER_BLOCKS[uncovered]);
                cell.set_style(
                    Style::default()
                        .fg(Color::DarkGray)
                        .bg(Color::White)
                        .remove_modifier(Modifier::DIM),
                );
            }
            _ => {
                // A partial thumb leaving below uses a dark background and a
                // white lower block for the visible thumb portion.
                cell.set_symbol(LOWER_BLOCKS[overlap]);
                cell.set_style(
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::DarkGray)
                        .remove_modifier(Modifier::DIM),
                );
            }
        }
    }
}
