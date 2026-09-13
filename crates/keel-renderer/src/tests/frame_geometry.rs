use super::popup;
use crate::{PatchOp, RenderContext, Renderer};

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
fn scroll_reservation_emits_scroll_once_and_keeps_the_effective_anchor() {
    let mut renderer = Renderer::new();
    let scene = popup(&["run git"]);
    let first = renderer
        .render(
            &scene,
            RenderContext::new(80, 5, 0)
                .terminal_rows(5)
                .scroll_rows(2)
                .full_repaint(true),
        )
        .unwrap();
    let second = renderer
        .render(
            &scene,
            RenderContext::new(80, 5, 0).terminal_rows(5).scroll_rows(2),
        )
        .unwrap();

    assert!(
        String::from_utf8(first.transaction.payload)
            .unwrap()
            .contains("\x1b[2S")
    );
    assert!(
        !String::from_utf8(second.transaction.payload)
            .unwrap()
            .contains("\x1b[2S")
    );
    assert_eq!(first.frame.anchor_row, 0);
    assert_eq!(second.frame.anchor_row, 0);
}

#[test]
fn releasing_scroll_reservation_scrolls_the_terminal_back_down() {
    let mut renderer = Renderer::new();
    let scene = popup(&["run git"]);
    renderer
        .render(
            &scene,
            RenderContext::new(80, 5, 0)
                .terminal_rows(5)
                .scroll_rows(2)
                .full_repaint(true),
        )
        .unwrap();
    let restored = renderer
        .render(&scene, RenderContext::new(80, 5, 0).terminal_rows(5))
        .unwrap();

    assert_eq!(restored.diff.scroll_rows_removed(), 2);
    assert!(
        restored
            .transaction
            .ops
            .contains(&PatchOp::ScrollDown { rows: 2 })
    );
    assert!(
        String::from_utf8(restored.transaction.payload)
            .unwrap()
            .contains("\x1b[2T")
    );
}
