use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
};

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
    let thumb_height = (track_height * visible_count).div_ceil(item_count).max(1);
    let max_thumb_top = track_height.saturating_sub(thumb_height);
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
        cell.set_symbol(" ");
        if (thumb_top..thumb_top + thumb_height).contains(&row) {
            cell.set_style(
                Style::default()
                    .bg(Color::White)
                    .remove_modifier(Modifier::DIM),
            );
        } else {
            cell.set_style(
                Style::default()
                    .bg(Color::White)
                    .add_modifier(Modifier::DIM),
            );
        }
    }
}
