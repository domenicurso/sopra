use super::snapshot;
use crate::render::{choose_popup_placement, popup_geometry, popup_layout};
use keel_ui::PopupPlacement;

#[test]
fn dismissing_overlay_does_not_change_the_host_line() {
    crate::keel_module_init();
    let mut output = [0_u8; 16 * 1024];
    let raw = snapshot(b"sh", 2);
    assert_eq!(
        crate::keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len()),
        0
    );
    super::set_test_suggestions();
    assert!(crate::keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len()) > 0);
    assert_eq!(crate::keel_module_dismiss_overlay(), 1);
    assert_eq!(crate::keel_module_has_suggestions(), 0);
    assert_eq!(crate::keel_module_move_selection(1), 0);
}

#[test]
fn unknown_native_abi_is_rejected_before_reading_host_memory() {
    crate::keel_module_init();
    let mut output = [0_u8; 4096];
    let mut raw = snapshot(b"sh", 2);
    raw.abi_version = crate::KEEL_NATIVE_ABI_VERSION + 1;
    assert_eq!(
        crate::keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len()),
        0
    );
}

#[test]
fn completion_protocol_wraps_candidates_in_the_current_line() {
    crate::keel_module_init();
    let mut output = [0_u8; 16 * 1024];
    let raw = snapshot(b"keel-test al", 12);
    assert_eq!(
        crate::keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len()),
        0
    );
    let payload = b"alpha\x1ffirst result\x1falpha\x1e";
    assert_eq!(
        unsafe { crate::keel_module_set_suggestions(payload.as_ptr(), payload.len(), 12) },
        1
    );
    assert_eq!(crate::keel_module_has_suggestions(), 1);
    assert_eq!(crate::keel_module_move_selection(1), 1);

    let mut replacement = [0_u8; 128];
    let length =
        crate::keel_module_selected_replacement(replacement.as_mut_ptr(), replacement.len());
    assert_eq!(&replacement[..length], b"keel-test alpha");
}

#[test]
fn popup_placement_prefers_below_then_above_and_shrinks_to_fit() {
    assert_eq!(
        choose_popup_placement(4, 6, 0),
        Some((PopupPlacement::Below, 4))
    );
    assert_eq!(
        choose_popup_placement(4, 1, 6),
        Some((PopupPlacement::Above, 4))
    );
    assert_eq!(
        choose_popup_placement(8, 3, 1),
        Some((PopupPlacement::Below, 3))
    );
    assert_eq!(choose_popup_placement(8, 2, 2), None);
}

#[test]
fn popup_geometry_aligns_inner_content_with_the_completion_token() {
    let (origin, anchor) = popup_geometry(20, 4, 24, 80);
    assert_eq!(origin + 2, 16);
    assert_eq!(origin + anchor, 20);

    let (clamped_origin, clamped_anchor) = popup_geometry(78, 4, 24, 80);
    assert_eq!(clamped_origin, 56);
    assert_eq!(clamped_anchor, 22);
}

#[test]
fn popup_layout_scrolls_the_host_when_below_space_is_short() {
    let layout = popup_layout(8, 2, 10).unwrap();
    assert_eq!(layout.placement, PopupPlacement::Below);
    assert_eq!(layout.height, 8);
    assert_eq!(layout.scroll_rows, 6);
}

#[test]
fn popup_layout_reserves_the_full_surface_near_the_bottom() {
    let layout = popup_layout(14, 8, 8).unwrap();
    assert_eq!(layout.placement, PopupPlacement::Below);
    assert_eq!(layout.height, 14);
    assert_eq!(layout.scroll_rows, 6);
}

#[test]
fn popup_layout_scrolls_the_full_surface_when_space_is_short() {
    let layout = popup_layout(14, 2, 20).unwrap();
    assert_eq!(layout.placement, PopupPlacement::Below);
    assert_eq!(layout.height, 14);
    assert_eq!(layout.scroll_rows, 12);
}

#[test]
fn pre_redraw_does_not_reverse_scroll_the_terminal() {
    crate::keel_module_init();
    let mut output = [0_u8; 16 * 1024];
    let mut raw = snapshot(b"sh", 2);
    raw.terminal_rows = 5;
    raw.cursor_row = 3;
    assert_eq!(
        crate::keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len()),
        0
    );
    super::set_test_suggestions();
    assert!(crate::keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len()) > 0);

    let size = crate::keel_module_before_redraw(output.as_mut_ptr(), output.len());
    assert_eq!(size, 0);
}
