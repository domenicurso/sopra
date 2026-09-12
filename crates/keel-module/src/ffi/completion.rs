use std::slice;

use keel_core::Suggestion;

use crate::abi::{
    COMPLETION_FIELD_SEPARATOR, COMPLETION_RECORD_SEPARATOR, MAX_COMPLETIONS, MAX_HOST_BYTES,
};
use crate::protocol::{completion_context, copy_payload, protocol_text, replace_token};
use crate::state::STATE;

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
