mod geometry;
mod overlay;

use crate::{KEEL_NATIVE_ABI_VERSION, KeelNativeHostSnapshot};

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
    let payload = b"shalpha\x1ffirst result\x1fshalpha\x1eshbeta\x1fsecond result\x1fshbeta\x1e";
    assert_eq!(
        unsafe { crate::keel_module_set_suggestions(payload.as_ptr(), payload.len(), 7) },
        1
    );
}
