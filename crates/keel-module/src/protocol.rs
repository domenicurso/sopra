use std::ptr;

use keel_core::{EditorBuffer, HostLine, HostSnapshot, ScreenPoint, TerminalSize};

use crate::abi::{KEEL_NATIVE_ABI_VERSION, KeelNativeHostSnapshot, MAX_HOST_BYTES};

#[derive(Debug)]
pub(crate) struct CompletionContext {
    pub(crate) token_start: usize,
    pub(crate) cursor_byte: usize,
}

pub(crate) fn completion_context(line: &str, cursor: usize) -> CompletionContext {
    let buffer = EditorBuffer::new(line, cursor);
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

pub(crate) fn replace_token(line: &str, context: &CompletionContext, candidate: &str) -> String {
    format!(
        "{}{}{}",
        &line[..context.token_start],
        candidate,
        &line[context.cursor_byte..]
    )
}

pub(crate) fn read_snapshot(raw: *const KeelNativeHostSnapshot) -> Option<HostSnapshot> {
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
    let bytes = unsafe { std::slice::from_raw_parts(pointer, length) };
    Some(String::from_utf8_lossy(bytes).into_owned())
}

pub(crate) fn protocol_text(bytes: &[u8]) -> Option<String> {
    if bytes
        .iter()
        .any(|byte| matches!(*byte, 0x00 | 0x1d | 0x1e | 0x1f | b'\r' | b'\n'))
    {
        return None;
    }
    Some(String::from_utf8_lossy(bytes).into_owned())
}

pub(crate) fn copy_payload(payload: &[u8], output: *mut u8, capacity: usize) -> usize {
    if payload.len() > capacity {
        return 0;
    }
    unsafe {
        ptr::copy_nonoverlapping(payload.as_ptr(), output, payload.len());
    }
    payload.len()
}
