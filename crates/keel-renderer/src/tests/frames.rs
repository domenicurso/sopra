use super::popup;
use crate::{PatchOp, RenderContext, Renderer};

#[test]
fn first_frame_paints_every_row_into_one_transaction() {
    let mut renderer = Renderer::new();
    let rendered = renderer
        .render(
            &popup(&["run git", "inspect git"]),
            RenderContext::new(80, 12, 3).full_repaint(true),
        )
        .unwrap();
    assert_eq!(
        rendered.diff.changed_rows,
        (0..rendered.frame.area.height).collect::<Vec<_>>()
    );
    assert_eq!(rendered.transaction.ops.first(), Some(&PatchOp::SaveCursor));
    let payload = String::from_utf8(rendered.transaction.payload).unwrap();
    let visible = super::ansi::strip_ansi(&payload);
    assert!(visible.contains("run git"));
    assert!(visible.contains("1/2; Tab to accept"));
    assert!(!visible.contains("Keel suggestions"));
}

#[test]
fn unchanged_frame_has_no_changed_rows_but_can_be_forced_full() {
    let mut renderer = Renderer::new();
    let scene = popup(&["run git"]);
    renderer
        .render(&scene, RenderContext::new(80, 12, 3).full_repaint(true))
        .unwrap();
    let unchanged = renderer
        .render(&scene, RenderContext::new(80, 12, 3))
        .unwrap();
    assert!(unchanged.diff.changed_rows.is_empty());
    assert!(!unchanged.diff.changed());
    assert!(unchanged.transaction.payload.is_empty());
    let forced = renderer
        .render(&scene, RenderContext::new(80, 12, 3).full_repaint(true))
        .unwrap();
    assert!(!forced.transaction.payload.is_empty());
}

#[test]
fn moving_surface_clears_old_anchor_and_repaints_new_anchor() {
    let mut renderer = Renderer::new();
    let scene = popup(&["run git"]);
    renderer
        .render(&scene, RenderContext::new(80, 12, 2).full_repaint(true))
        .unwrap();
    let moved = renderer
        .render(&scene, RenderContext::new(80, 12, 7))
        .unwrap();

    assert!(moved.diff.origin_changed());
    assert!(moved.transaction.ops.iter().any(|op| {
        matches!(
            op,
            PatchOp::ClearSurface {
                origin_column: 2,
                ..
            }
        )
    }));
    let payload = String::from_utf8(moved.transaction.payload).unwrap();
    assert!(payload.contains("\x1b[3G"));
    assert!(payload.contains("\x1b[8G"));
}

#[test]
fn moving_surface_restores_host_cursor_before_clearing_the_old_offset() {
    let mut renderer = Renderer::new();
    let scene = popup(&["run git"]);
    renderer
        .render(
            &scene,
            RenderContext::new(80, 12, 2)
                .row_offset(-4)
                .full_repaint(true),
        )
        .unwrap();
    let moved = renderer
        .render(
            &scene,
            RenderContext::new(80, 12, 7)
                .row_offset(2)
                .full_repaint(false),
        )
        .unwrap();
    let payload = String::from_utf8(moved.transaction.payload).unwrap();
    let old_clear = payload.find("\x1b8\x1b[4A").unwrap();
    let new_paint = payload.find("\x1b8\x1b[2B").unwrap();
    assert!(old_clear < new_paint);
}

#[test]
fn shrinking_surface_clears_old_rows() {
    let mut renderer = Renderer::new();
    let previous = renderer
        .render(
            &popup(&["one", "two", "three"]),
            RenderContext::new(80, 12, 0),
        )
        .unwrap();
    let next = renderer
        .render(&popup(&["one"]), RenderContext::new(80, 12, 0))
        .unwrap();
    assert_eq!(
        next.diff.cleared_rows,
        (next.frame.area.height..previous.frame.area.height).collect::<Vec<_>>()
    );
    let clear = renderer.clear_previous().unwrap();
    assert!(String::from_utf8(clear).unwrap().contains("\x1b["));
}
