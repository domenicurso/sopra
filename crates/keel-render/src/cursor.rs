use keel_core::{CursorStyle, SceneCursor, VisualCursor};

pub fn visual_cursor(
    row: u16,
    column: u16,
    cursor: Option<SceneCursor>,
) -> Option<VisualCursor> {
    cursor.and_then(|cursor| {
        if !cursor.visible {
            return None;
        }

        Some(VisualCursor {
            row,
            column,
            style: match cursor.style {
                CursorStyle::Block => CursorStyle::Block,
                CursorStyle::Beam => CursorStyle::Beam,
            },
            visible: true,
            alpha: cursor.alpha,
        })
    })
}
