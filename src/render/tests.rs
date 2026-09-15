use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
};

use super::ansi::cell_at;

#[test]
fn cells_outside_a_frame_are_empty() {
    let frame = Buffer::empty(Rect::new(0, 0, 2, 1));
    assert_eq!(cell_at(&frame, 9, 9), ratatui::buffer::Cell::EMPTY);
}

#[test]
fn ratatui_cells_are_the_renderer_contract() {
    let mut frame = Buffer::empty(Rect::new(0, 0, 2, 1));
    frame[(0, 0)]
        .set_symbol("K")
        .set_style(Style::default().fg(Color::Cyan));
    assert_eq!(frame[(0, 0)].symbol(), "K");
    assert_eq!(frame[(0, 0)].fg, Color::Cyan);
}

#[test]
fn moves_are_relative_to_the_saved_origin() {
    let mut output = Vec::new();
    super::ansi::move_to(&mut output, 5, 2);
    assert_eq!(output, b"\x1b[u\x1b[2B\x1b[6G");
}
