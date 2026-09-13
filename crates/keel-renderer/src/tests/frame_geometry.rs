use super::popup;
use crate::{RenderContext, Renderer};

#[test]
fn terminal_resize_forces_a_full_repaint_even_when_the_scene_is_unchanged() {
    let mut renderer = Renderer::new();
    let scene = popup(&["run git"]);
    renderer
        .render(
            &scene,
            RenderContext::new(80, 12, 0)
                .terminal_rows(24)
                .full_repaint(true),
        )
        .unwrap();
    let resized = renderer
        .render(
            &scene,
            RenderContext::new(80, 12, 0)
                .terminal_rows(30)
                .full_repaint(false),
        )
        .unwrap();

    assert!(resized.diff.terminal_changed);
    assert_eq!(
        resized.diff.changed_rows,
        (0..resized.frame.area.height).collect::<Vec<_>>()
    );
    assert!(!resized.transaction.payload.is_empty());
}

#[test]
fn scroll_reservation_writes_scrollback_lines_and_keeps_the_post_scroll_anchor() {
    let mut renderer = Renderer::new();
    let scene = popup(&["run git"]);
    let first = renderer
        .render(
            &scene,
            RenderContext::new(80, 5, 0)
                .terminal_rows(8)
                .cursor_row(4)
                .scroll_rows(2)
                .full_repaint(true),
        )
        .unwrap();
    let second = renderer
        .render(
            &scene,
            RenderContext::new(80, 5, 0).terminal_rows(8).cursor_row(2),
        )
        .unwrap();

    let first_payload = String::from_utf8(first.transaction.payload).unwrap();
    assert_eq!(first.transaction.scroll_rows, 2);
    assert_eq!(first.frame.cursor_row, 2);
    assert_eq!(first.frame.anchor_row, 3);
    assert_eq!(first_payload.matches("\x1bD").count(), 2);
    assert!(!first_payload.contains('\n'));
    assert!(!first_payload.contains("\x1b[2S"));
    assert!(!first_payload.contains("\x1b[2T"));
    assert_eq!(second.frame.anchor_row, 3);
    assert!(second.transaction.payload.is_empty());
}

#[test]
fn clearing_scrolled_surface_never_reverse_scrolls_the_terminal() {
    let mut renderer = Renderer::new();
    let scene = popup(&["run git"]);
    renderer
        .render(
            &scene,
            RenderContext::new(80, 5, 0)
                .terminal_rows(8)
                .cursor_row(4)
                .scroll_rows(2)
                .full_repaint(true),
        )
        .unwrap();
    let clear = String::from_utf8(renderer.clear_previous().unwrap()).unwrap();

    assert!(!clear.contains("\x1b[2S"));
    assert!(!clear.contains("\x1b[2T"));
}
