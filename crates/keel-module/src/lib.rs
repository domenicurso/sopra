use std::{cell::RefCell, ptr, slice};

use keel_core::{
    AppState, EditorEvent, HostLine, HostSnapshot, SUGGESTION_VIEWPORT_ROWS, ScreenPoint,
    Suggestion, TerminalSize,
};
use keel_renderer::{RenderContext, Renderer};
use keel_scheduler::{FrameClock, InvalidationReason};
use keel_ui::{Constraints, MAX_VISIBLE_ITEMS, PopupItem, PopupPlacement, Scene, SuggestionPopup};

const MAX_HOST_BYTES: usize = 256 * 1024;
const MAX_COMPLETIONS: usize = 512;
const MAX_POPUP_ROWS: u16 = MAX_VISIBLE_ITEMS as u16 + 2;
const POPUP_MIN_ROWS: u16 = 3;
const COMPLETION_FIELD_SEPARATOR: u8 = 0x1f;
const COMPLETION_RECORD_SEPARATOR: u8 = 0x1e;
pub const KEEL_NATIVE_ABI_VERSION: u32 = 1;

#[derive(Debug)]
struct CompletionContext {
    token_start: usize,
    cursor_byte: usize,
}

fn completion_context(line: &str, cursor: usize) -> CompletionContext {
    let buffer = keel_core::EditorBuffer::new(line, cursor);
    let cursor_byte = buffer.cursor_byte_offset();
    let prefix = &line[..cursor_byte];
    let token_start = prefix
        .char_indices()
        .rev()
        .find(|(_, character)| character.is_whitespace())
        .map_or(0, |(offset, character)| offset + character.len_utf8());
    CompletionContext {
        token_start,
        cursor_byte,
    }
}

fn replace_token(line: &str, context: &CompletionContext, candidate: &str) -> String {
    format!(
        "{}{}{}",
        &line[..context.token_start],
        candidate,
        &line[context.cursor_byte..]
    )
}

#[repr(C)]
pub struct KeelNativeHostSnapshot {
    pub abi_version: u32,
    pub buffer: *const u8,
    pub buffer_len: usize,
    pub cursor_units: usize,
    pub terminal_columns: u16,
    pub terminal_rows: u16,
    pub cursor_column: u16,
    pub cursor_row: u16,
    pub cwd: *const u8,
    pub cwd_len: usize,
    pub keymap: *const u8,
    pub keymap_len: usize,
    pub last_status: i32,
    pub redisplay_generation: u64,
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct KeelNativeStats {
    pub redraws: u64,
    pub clears: u64,
    pub last_payload_bytes: usize,
    pub last_rows: u16,
}

struct ModuleState {
    app: AppState,
    renderer: Renderer,
    clock: FrameClock,
    stats: KeelNativeStats,
    completion_ms: u64,
}

impl ModuleState {
    fn new() -> Self {
        Self {
            app: AppState::default(),
            renderer: Renderer::new(),
            clock: FrameClock::new(),
            stats: KeelNativeStats::default(),
            completion_ms: 0,
        }
    }

    fn reset(&mut self) {
        *self = Self::new();
    }
}

thread_local! {
    static STATE: RefCell<ModuleState> = RefCell::new(ModuleState::new());
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_module_init() {
    STATE.with(|state| state.borrow_mut().reset());
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_module_shutdown() {
    STATE.with(|state| state.borrow_mut().reset());
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_module_before_redraw(output: *mut u8, capacity: usize) -> usize {
    if output.is_null() {
        return 0;
    }
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let payload = state.renderer.clear_previous().unwrap_or_default();
        state.stats.clears = state.stats.clears.saturating_add(1);
        state.stats.last_payload_bytes = payload.len();
        copy_payload(&payload, output, capacity)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_module_after_redraw(
    snapshot: *const KeelNativeHostSnapshot,
    output: *mut u8,
    capacity: usize,
) -> usize {
    if snapshot.is_null() || output.is_null() {
        return 0;
    }

    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(snapshot) = read_snapshot(snapshot) else {
            return 0;
        };
        state
            .clock
            .invalidate(InvalidationReason::HostRedisplay, std::time::Instant::now());
        state.app.apply(EditorEvent::Redisplay(snapshot.clone()));
        let completion_ms = state.completion_ms;

        let Some(probe) = build_scene(&state.app, 0, PopupPlacement::Below, completion_ms) else {
            state.clock.rendered(std::time::Instant::now());
            state.stats.last_payload_bytes = 0;
            state.stats.last_rows = 0;
            return 0;
        };

        let columns = snapshot.terminal.columns.max(1);
        let natural = probe.measure(Constraints::new(columns, u16::MAX));
        let requested_height = natural.height.clamp(POPUP_MIN_ROWS, MAX_POPUP_ROWS);
        let below = snapshot
            .terminal
            .rows
            .saturating_sub(snapshot.cursor.row.saturating_add(1));
        let above = snapshot.cursor.row;
        let Some((placement, height)) = choose_popup_placement(requested_height, below, above)
        else {
            state.clock.rendered(std::time::Instant::now());
            state.stats.last_payload_bytes = 0;
            state.stats.last_rows = 0;
            return 0;
        };
        let width = natural.width.min(columns).max(1);
        let (origin, anchor) = popup_geometry(
            snapshot.cursor.column,
            state.app.completion_token_width(),
            width,
            columns,
        );
        let Some(scene) = build_scene(&state.app, anchor, placement, completion_ms) else {
            return 0;
        };
        let row_offset = match placement {
            PopupPlacement::Below => 1,
            PopupPlacement::Above => -(height as i16),
        };
        let context = RenderContext::new(columns, height, origin)
            .row_offset(row_offset)
            .full_repaint(true);
        let rendered = match state.renderer.render(&scene, context) {
            Ok(rendered) => rendered,
            Err(_) => return 0,
        };
        let payload = &rendered.transaction.payload;
        state.stats.redraws = state.stats.redraws.saturating_add(1);
        state.stats.last_payload_bytes = payload.len();
        state.stats.last_rows = rendered.frame.area.height;
        state.clock.rendered(std::time::Instant::now());
        copy_payload(payload, output, capacity)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_module_line_init() {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.app = AppState::default();
        state.clock = FrameClock::new();
        state.completion_ms = 0;
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_module_line_finish(output: *mut u8, capacity: usize) -> usize {
    if output.is_null() {
        return 0;
    }
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let payload = state.renderer.clear_previous().unwrap_or_default();
        state.app = AppState::default();
        state.clock = FrameClock::new();
        state.stats.clears = state.stats.clears.saturating_add(1);
        state.stats.last_payload_bytes = payload.len();
        state.stats.last_rows = 0;
        copy_payload(&payload, output, capacity)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_module_reset() {
    STATE.with(|state| state.borrow_mut().reset());
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_module_stats() -> KeelNativeStats {
    STATE.with(|state| state.borrow().stats)
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_module_has_suggestions() -> i32 {
    STATE.with(|state| {
        if state.borrow().app.suggestions_visible() {
            1
        } else {
            0
        }
    })
}

/// Install the completion engine's latest result without asking ZLE to render it.
///
/// The payload is a sequence of `label\x1fdetail\x1fcandidate\x1e` records. The
/// candidate is the completion token, so the module can derive the authoritative
/// full line from the current Rust-owned snapshot.
///
/// # Safety
///
/// When `payload` is non-null, it must point to `length` readable bytes for the
/// duration of the call. A null pointer is accepted only with a zero length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn keel_module_set_suggestions(
    payload: *const u8,
    length: usize,
    completion_ms: u64,
) -> i32 {
    if payload.is_null() || length > MAX_HOST_BYTES {
        return 0;
    }

    let bytes = unsafe { slice::from_raw_parts(payload, length) };
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let context = completion_context(state.app.buffer.text(), state.app.buffer.cursor());
        let mut suggestions = Vec::with_capacity(MAX_COMPLETIONS.min(8));

        for record in bytes.split(|byte| *byte == COMPLETION_RECORD_SEPARATOR) {
            if record.is_empty() || suggestions.len() == MAX_COMPLETIONS {
                break;
            }
            let mut fields = record.splitn(3, |byte| *byte == COMPLETION_FIELD_SEPARATOR);
            let Some(label) = protocol_text(fields.next().unwrap_or_default()) else {
                continue;
            };
            let Some(detail) = protocol_text(fields.next().unwrap_or_default()) else {
                continue;
            };
            let Some(candidate) = protocol_text(fields.next().unwrap_or_default()) else {
                continue;
            };
            if label.is_empty() || candidate.is_empty() {
                continue;
            }

            let replacement = replace_token(state.app.buffer.text(), &context, &candidate);
            suggestions.push(Suggestion::with_replacement(label, detail, replacement));
        }

        state.completion_ms = completion_ms;
        state.app.set_suggestions(suggestions);
        1
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_module_move_selection(delta: i32) -> i32 {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if !state.app.suggestions_visible() {
            return 0;
        }
        state.app.move_selection(delta as isize);
        1
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_module_selected_replacement(output: *mut u8, capacity: usize) -> usize {
    if output.is_null() {
        return 0;
    }
    STATE.with(|state| {
        let state = state.borrow();
        let Some(replacement) = state.app.selected_replacement() else {
            return 0;
        };
        copy_payload(replacement.as_bytes(), output, capacity)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_module_dismiss_overlay() -> i32 {
    STATE.with(|state| {
        if state.borrow_mut().app.dismiss_overlay() {
            1
        } else {
            0
        }
    })
}

/// # Safety
///
/// When `line` is non-null, it must point to `length` readable bytes for the
/// duration of the call. A null pointer is accepted only with a zero length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn keel_module_suppress_overlay_for_line(line: *const u8, length: usize) {
    if line.is_null() || length > MAX_HOST_BYTES {
        return;
    }
    let text = unsafe { slice::from_raw_parts(line, length) };
    let text = String::from_utf8_lossy(text).into_owned();
    STATE.with(|state| {
        state.borrow_mut().app.suppress_overlay_for_text(text);
    });
}

fn build_scene(
    app: &AppState,
    anchor: u16,
    placement: PopupPlacement,
    completion_ms: u64,
) -> Option<Scene> {
    if !app.suggestions_visible() {
        return None;
    }
    let items = app
        .suggestions
        .iter()
        .map(|suggestion| PopupItem::new(&suggestion.label, &suggestion.detail))
        .collect::<Vec<_>>();
    let footer = format!(
        "{}/{}; {}ms",
        app.selected_suggestion
            .map_or_else(|| "0".to_string(), |selected| (selected + 1).to_string()),
        items.len(),
        completion_ms,
    );
    Some(Scene::new(
        SuggestionPopup::new(footer, items, app.selected_suggestion)
            .query(app.completion_query())
            .anchor_column(anchor)
            .placement(placement)
            .viewport(
                app.suggestion_viewport_start(),
                SUGGESTION_VIEWPORT_ROWS.min(MAX_VISIBLE_ITEMS),
            ),
    ))
}

fn choose_popup_placement(
    requested_height: u16,
    below: u16,
    above: u16,
) -> Option<(PopupPlacement, u16)> {
    if below >= requested_height {
        return Some((PopupPlacement::Below, requested_height));
    }
    if above >= requested_height {
        return Some((PopupPlacement::Above, requested_height));
    }

    let below_height = below.min(requested_height);
    let above_height = above.min(requested_height);
    if below_height >= POPUP_MIN_ROWS || above_height >= POPUP_MIN_ROWS {
        if below_height >= above_height {
            Some((PopupPlacement::Below, below_height))
        } else {
            Some((PopupPlacement::Above, above_height))
        }
    } else {
        None
    }
}

fn popup_geometry(cursor_column: u16, token_width: u16, width: u16, columns: u16) -> (u16, u16) {
    let max_origin = columns.saturating_sub(width);
    let token_start = cursor_column.saturating_sub(token_width);
    let origin = token_start.saturating_sub(2).min(max_origin);
    let anchor = cursor_column
        .saturating_sub(origin)
        .clamp(1, width.saturating_sub(2).max(1));
    (origin, anchor)
}

fn read_snapshot(raw: *const KeelNativeHostSnapshot) -> Option<HostSnapshot> {
    // The C shim owns these buffers for the duration of the call.
    let raw = unsafe { &*raw };
    if raw.abi_version != KEEL_NATIVE_ABI_VERSION {
        return None;
    }
    let buffer = read_utf8(raw.buffer, raw.buffer_len)?;
    let cwd = read_utf8(raw.cwd, raw.cwd_len)?;
    let keymap = read_utf8(raw.keymap, raw.keymap_len)?;
    Some(HostSnapshot {
        line: HostLine::from_codepoint_cursor(buffer, raw.cursor_units),
        terminal: TerminalSize::new(raw.terminal_columns, raw.terminal_rows),
        cursor: ScreenPoint {
            column: raw.cursor_column,
            row: raw.cursor_row,
        },
        cwd,
        keymap,
        last_status: raw.last_status,
        redisplay_generation: raw.redisplay_generation,
    })
}

fn read_utf8(pointer: *const u8, length: usize) -> Option<String> {
    if pointer.is_null() || length > MAX_HOST_BYTES {
        return None;
    }
    let bytes = unsafe { slice::from_raw_parts(pointer, length) };
    Some(String::from_utf8_lossy(bytes).into_owned())
}

fn protocol_text(bytes: &[u8]) -> Option<String> {
    if bytes
        .iter()
        .any(|byte| matches!(*byte, 0x00 | 0x1d | 0x1e | 0x1f | b'\r' | b'\n'))
    {
        return None;
    }
    Some(String::from_utf8_lossy(bytes).into_owned())
}

fn copy_payload(payload: &[u8], output: *mut u8, capacity: usize) -> usize {
    if payload.len() > capacity {
        return 0;
    }
    unsafe {
        ptr::copy_nonoverlapping(payload.as_ptr(), output, payload.len());
    }
    payload.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(text: &[u8], cursor_column: u16) -> KeelNativeHostSnapshot {
        KeelNativeHostSnapshot {
            abi_version: KEEL_NATIVE_ABI_VERSION,
            buffer: text.as_ptr(),
            buffer_len: text.len(),
            cursor_units: text.len(),
            terminal_columns: 80,
            terminal_rows: 24,
            cursor_column,
            cursor_row: 0,
            cwd: b"/tmp".as_ptr(),
            cwd_len: 4,
            keymap: b"main".as_ptr(),
            keymap_len: 4,
            last_status: 0,
            redisplay_generation: 1,
        }
    }

    fn set_test_suggestions() {
        let payload = b"alpha\x1ffirst result\x1falpha\x1ebeta\x1fsecond result\x1fbeta\x1e";
        assert_eq!(
            unsafe { keel_module_set_suggestions(payload.as_ptr(), payload.len(), 7) },
            1
        );
    }

    #[test]
    fn empty_input_has_no_overlay() {
        keel_module_init();
        let mut output = [0_u8; 4096];
        let raw = snapshot(b"", 0);
        assert_eq!(
            keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len()),
            0
        );
    }

    #[test]
    fn non_empty_input_renders_the_popup() {
        keel_module_init();
        let mut output = [0_u8; 16 * 1024];
        let raw = snapshot(b"sh", 2);
        assert_eq!(
            keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len()),
            0
        );
        set_test_suggestions();
        let size = keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len());
        assert!(size > 0);
        let rendered = String::from_utf8_lossy(&output[..size]);
        assert!(rendered.contains("alp") && rendered.contains("h"));
    }

    #[test]
    fn completion_protocol_accepts_option_records_with_descriptions() {
        keel_module_init();
        let mut output = [0_u8; 16 * 1024];
        let raw = snapshot(b"command --", 10);
        assert_eq!(
            keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len()),
            0
        );
        let payload = b"--verbose\x1fshow verbose output\x1f--verbose\x1e--format\x1fselect output format\x1f--format\x1e";
        assert_eq!(
            unsafe { keel_module_set_suggestions(payload.as_ptr(), payload.len(), 4) },
            1
        );
        assert_eq!(keel_module_has_suggestions(), 1);
    }

    #[test]
    fn pre_redraw_clears_the_previous_surface() {
        keel_module_init();
        let mut output = [0_u8; 16 * 1024];
        let raw = snapshot(b"sh", 2);
        assert_eq!(
            keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len()),
            0
        );
        set_test_suggestions();
        assert!(keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len()) > 0);
        let size = keel_module_before_redraw(output.as_mut_ptr(), output.len());
        assert!(size > 0);
        assert!(String::from_utf8_lossy(&output[..size]).contains("\x1b["));
    }

    #[test]
    fn selected_replacement_and_overlay_suppression_are_host_safe() {
        keel_module_init();
        let mut output = [0_u8; 16 * 1024];
        let initial = snapshot(b"sh", 2);
        assert_eq!(
            keel_module_after_redraw(&initial, output.as_mut_ptr(), output.len()),
            0
        );
        set_test_suggestions();
        assert!(keel_module_after_redraw(&initial, output.as_mut_ptr(), output.len()) > 0);
        assert_eq!(keel_module_has_suggestions(), 1);
        assert_eq!(keel_module_move_selection(1), 1);

        let mut replacement = [0_u8; 128];
        let replacement_length =
            keel_module_selected_replacement(replacement.as_mut_ptr(), replacement.len());
        assert!(replacement_length > 0);
        unsafe {
            keel_module_suppress_overlay_for_line(replacement.as_ptr(), replacement_length);
        }

        let next = snapshot(
            &replacement[..replacement_length],
            replacement_length as u16,
        );
        assert_eq!(
            keel_module_after_redraw(&next, output.as_mut_ptr(), output.len()),
            0
        );
        assert_eq!(keel_module_has_suggestions(), 0);
    }

    #[test]
    fn dismissing_overlay_does_not_change_the_host_line() {
        keel_module_init();
        let mut output = [0_u8; 16 * 1024];
        let raw = snapshot(b"sh", 2);
        assert_eq!(
            keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len()),
            0
        );
        set_test_suggestions();
        assert!(keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len()) > 0);
        assert_eq!(keel_module_dismiss_overlay(), 1);
        assert_eq!(keel_module_has_suggestions(), 0);
        assert_eq!(keel_module_move_selection(1), 0);
    }

    #[test]
    fn unknown_native_abi_is_rejected_before_reading_host_memory() {
        keel_module_init();
        let mut output = [0_u8; 4096];
        let mut raw = snapshot(b"sh", 2);
        raw.abi_version = KEEL_NATIVE_ABI_VERSION + 1;
        assert_eq!(
            keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len()),
            0
        );
    }

    #[test]
    fn completion_protocol_wraps_candidates_in_the_current_line() {
        keel_module_init();
        let mut output = [0_u8; 16 * 1024];
        let raw = snapshot(b"keel-test al", 12);
        assert_eq!(
            keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len()),
            0
        );
        let payload = b"alpha\x1ffirst result\x1falpha\x1e";
        assert_eq!(
            unsafe { keel_module_set_suggestions(payload.as_ptr(), payload.len(), 12) },
            1
        );
        assert_eq!(keel_module_has_suggestions(), 1);
        assert_eq!(keel_module_move_selection(1), 1);

        let mut replacement = [0_u8; 128];
        let length = keel_module_selected_replacement(replacement.as_mut_ptr(), replacement.len());
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
}
