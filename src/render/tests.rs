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
fn initial_repaint_emits_only_occupied_cells() {
    let mut renderer = super::Renderer::editor();
    renderer.origin_row = Some(0);
    let mut frame = Buffer::empty(Rect::new(0, 0, 120, 40));
    frame[(0, 0)].set_symbol("$");
    let mut output = Vec::new();

    let changed = renderer.paint_diff(&frame, Rect::default(), &[], &mut output);

    assert_eq!(changed, 1);
    assert!(output.ends_with(b"$"));
    assert!(output.len() < 64);
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
fn moves_use_absolute_terminal_coordinates() {
    let mut output = Vec::new();
    super::ansi::move_to(&mut output, 5, 2);
    assert_eq!(output, b"\x1b[3;6H");
}

#[test]
fn scrolling_makes_room_only_for_the_missing_popup_rows() {
    assert_eq!(super::rows_to_scroll(39, 15, 40), 14);
    assert_eq!(super::rows_to_scroll(35, 15, 40), 10);
    assert_eq!(super::rows_to_scroll(0, 15, 40), 0);
}

#[test]
fn scrolling_updates_the_absolute_origin_after_moving_the_viewport() {
    let mut renderer = super::Renderer {
        origin_row: Some(35),
        ..super::Renderer::default()
    };
    let mut output = Vec::new();
    renderer.ensure_space(15, 40, &mut output);
    assert_eq!(renderer.origin_row, Some(25));
    assert_eq!(
        output,
        [b"\x1b[40;1H".as_slice(), &b"\r\n".repeat(10)].concat()
    );
}

#[test]
fn resize_signal_forces_a_repaint_when_dimensions_return_to_the_previous_size() {
    use ratatui::{buffer::Buffer, layout::Rect};

    let area = Rect::new(0, 0, 120, 40);
    let renderer = super::Renderer {
        previous: Some(Buffer::empty(area)),
        resize_epoch: Some(8),
        ..super::Renderer::default()
    };

    assert!(renderer.resize_detected(area, 9));
    assert!(!renderer.resize_detected(area, 8));
}

#[test]
fn resize_origin_tracks_the_cursor_instead_of_the_right_prompt() {
    let mut previous = Buffer::empty(Rect::new(0, 0, 180, 24));
    previous[(0, 0)].set_symbol("$");
    previous[(179, 0)].set_symbol("t");
    let renderer = super::Renderer {
        previous: Some(previous),
        previous_cursor: Some(crate::input::CursorPosition { column: 20, row: 0 }),
        ..super::Renderer::default()
    };

    let cursor = |row, column| crate::input::CursorPosition { row, column };
    assert_eq!(renderer.resize_origin(cursor(18, 20), 0, 40), 18);
    assert_eq!(renderer.resize_origin(cursor(18, 9), 0, 10), 18);
    assert_eq!(renderer.resize_origin(cursor(20, 0), 0, 10), 18);
}
