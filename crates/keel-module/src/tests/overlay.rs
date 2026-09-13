use super::{set_test_suggestions, snapshot};

#[test]
fn empty_input_has_no_overlay() {
    crate::keel_module_init();
    let mut output = [0_u8; 4096];
    let raw = snapshot(b"", 0);
    assert_eq!(
        crate::keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len()),
        0
    );
}

#[test]
fn non_empty_input_renders_the_popup() {
    crate::keel_module_init();
    let mut output = [0_u8; 16 * 1024];
    let raw = snapshot(b"sh", 2);
    assert_eq!(
        crate::keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len()),
        0
    );
    set_test_suggestions();
    let size = crate::keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len());
    assert!(size > 0);
    let rendered = String::from_utf8_lossy(&output[..size]);
    assert!(rendered.contains("alp") && rendered.contains("h"));
}

#[test]
fn completion_protocol_accepts_option_records_with_descriptions() {
    crate::keel_module_init();
    let mut output = [0_u8; 16 * 1024];
    let raw = snapshot(b"command --", 10);
    assert_eq!(
        crate::keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len()),
        0
    );
    let payload = b"--verbose\x1fshow verbose output\x1f--verbose\x1e--format\x1fselect output format\x1f--format\x1e";
    assert_eq!(
        unsafe { crate::keel_module_set_suggestions(payload.as_ptr(), payload.len(), 4) },
        1
    );
    assert_eq!(crate::keel_module_has_suggestions(), 1);
}

#[test]
fn cached_completion_source_can_be_refreshed_without_a_new_payload() {
    crate::keel_module_init();
    let mut output = [0_u8; 16 * 1024];
    let raw = snapshot(b"keel-test al", 12);
    assert_eq!(
        crate::keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len()),
        0
    );
    let payload = b"alpha\x1ffirst result\x1falpha\x1e";
    assert_eq!(
        unsafe { crate::keel_module_set_suggestions(payload.as_ptr(), payload.len(), 4) },
        1
    );
    assert_eq!(crate::keel_module_refresh_suggestions(0), 1);
    assert_eq!(crate::keel_module_has_suggestions(), 1);
}

#[test]
fn selected_candidate_uses_the_latest_host_line() {
    crate::keel_module_init();
    let mut output = [0_u8; 16 * 1024];
    let initial = snapshot(b"keel-test al", 12);
    assert_eq!(
        crate::keel_module_after_redraw(&initial, output.as_mut_ptr(), output.len()),
        0
    );
    let payload = b"alpha\x1ffirst result\x1falpha\x1e";
    assert_eq!(
        unsafe { crate::keel_module_set_suggestions(payload.as_ptr(), payload.len(), 0) },
        1
    );

    let latest = snapshot(b"keel-test alp", 13);
    assert!(crate::keel_module_after_redraw(&latest, output.as_mut_ptr(), output.len()) > 0);
    assert_eq!(crate::keel_module_move_selection(1), 1);

    let mut replacement = [0_u8; 128];
    let length =
        crate::keel_module_selected_replacement(replacement.as_mut_ptr(), replacement.len());
    assert_eq!(&replacement[..length], b"keel-test alpha");
}

#[test]
fn pre_redraw_keeps_the_previous_surface_for_incremental_diffing() {
    crate::keel_module_init();
    let mut output = [0_u8; 16 * 1024];
    let raw = snapshot(b"sh", 2);
    assert_eq!(
        crate::keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len()),
        0
    );
    set_test_suggestions();
    assert!(crate::keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len()) > 0);
    let size = crate::keel_module_before_redraw(output.as_mut_ptr(), output.len());
    assert_eq!(size, 0);
    assert_eq!(
        crate::keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len()),
        0
    );
}

#[test]
fn selected_replacement_and_overlay_suppression_are_host_safe() {
    crate::keel_module_init();
    let mut output = [0_u8; 16 * 1024];
    let initial = snapshot(b"sh", 2);
    assert_eq!(
        crate::keel_module_after_redraw(&initial, output.as_mut_ptr(), output.len()),
        0
    );
    set_test_suggestions();
    assert!(crate::keel_module_after_redraw(&initial, output.as_mut_ptr(), output.len()) > 0);
    assert_eq!(crate::keel_module_has_suggestions(), 1);
    assert_eq!(crate::keel_module_move_selection(1), 1);

    let mut replacement = [0_u8; 128];
    let replacement_length =
        crate::keel_module_selected_replacement(replacement.as_mut_ptr(), replacement.len());
    assert!(replacement_length > 0);
    unsafe {
        crate::keel_module_suppress_overlay_for_line(replacement.as_ptr(), replacement_length);
    }

    let next = snapshot(
        &replacement[..replacement_length],
        replacement_length as u16,
    );
    let size = crate::keel_module_after_redraw(&next, output.as_mut_ptr(), output.len());
    assert!(size > 0);
    assert!(String::from_utf8_lossy(&output[..size]).contains("\x1b["));
    assert_eq!(crate::keel_module_has_suggestions(), 0);
}
