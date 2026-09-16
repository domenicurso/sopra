use crate::{editor::EditorState, input::TerminalSize};

use super::MAX_OVERLAY_ITEMS;

pub(super) struct OverlayLayout {
    pub(super) row: u16,
    pub(super) height: u16,
    pub(super) above: bool,
}

pub(super) fn for_editor(
    editor: &EditorState,
    size: TerminalSize,
    row: u16,
) -> Option<OverlayLayout> {
    if !editor.overlay_visible() || editor.visible_items().is_empty() {
        return None;
    }
    let requested = editor.visible_items().len().min(MAX_OVERLAY_ITEMS) as u16 + 2;
    let below = size.rows.saturating_sub(row.saturating_add(2));
    let above = row.saturating_sub(1);
    let (above, height) = placement(requested, below, above)?;
    let overlay_row = if above {
        row.saturating_sub(height.saturating_add(1))
    } else {
        row.saturating_add(2)
    };
    Some(OverlayLayout {
        row: overlay_row,
        height,
        above,
    })
}

fn placement(requested: u16, below: u16, above: u16) -> Option<(bool, u16)> {
    if below >= requested {
        return Some((false, requested));
    }
    if above >= requested {
        return Some((true, requested));
    }
    let below_height = below.min(requested);
    let above_height = above.min(requested);
    if below_height < 3 && above_height < 3 {
        return None;
    }
    Some(if below_height >= above_height {
        (false, below_height)
    } else {
        (true, above_height)
    })
}

#[cfg(test)]
mod tests {
    use super::placement;

    #[test]
    fn prefers_below_then_above_then_the_larger_partial_space() {
        assert_eq!(placement(14, 14, 2), Some((false, 14)));
        assert_eq!(placement(14, 2, 14), Some((true, 14)));
        assert_eq!(placement(14, 7, 5), Some((false, 7)));
        assert_eq!(placement(14, 2, 2), None);
    }
}
