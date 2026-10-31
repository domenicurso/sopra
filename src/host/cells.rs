use ratatui::{
    buffer::Buffer,
    style::{Color, Modifier, Style},
};
pub(super) fn copy_screen(buffer: &mut Buffer, screen: &vt100::Screen) {
    for row in 0..buffer.area.height {
        for column in 0..buffer.area.width {
            let Some(cell) = screen
                .cell(row, column)
                .filter(|cell| !cell.is_wide_continuation())
            else {
                continue;
            };
            let Some(target) = buffer.cell_mut((column, row)) else {
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
