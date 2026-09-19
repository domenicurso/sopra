use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
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
fn region_highlight_uses_the_ratatui_style_model() {
    let style = Style::default()
        .fg(Color::LightMagenta)
        .add_modifier(Modifier::UNDERLINED);
    assert_eq!(
        super::ansi::region_highlight_style(style).as_deref(),
        Some("fg=13,underline")
    );
}

#[test]
fn moves_are_relative_to_the_saved_origin() {
    let mut output = Vec::new();
    super::ansi::move_to(&mut output, 5, 2);
    assert_eq!(output, b"\x1b[u\x1b[2B\x1b[6G");
}

#[test]
fn scrolling_makes_room_only_for_the_missing_popup_rows() {
    assert_eq!(super::rows_to_scroll(39, 15, 40), 14);
    assert_eq!(super::rows_to_scroll(35, 15, 40), 10);
    assert_eq!(super::rows_to_scroll(0, 15, 40), 0);
}

#[test]
fn scrolling_resets_the_saved_origin_after_moving_the_viewport() {
    let mut renderer = super::Renderer {
        origin_row: Some(35),
        ..super::Renderer::default()
    };
    let mut output = Vec::new();
    renderer.ensure_space(15, 40, &mut output);
    assert_eq!(renderer.origin_row, Some(25));
    assert_eq!(output, b"\x1b[u\x1b[10S\x1b[10A\r\x1b[s".to_vec());
}
