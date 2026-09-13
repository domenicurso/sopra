use keel_scheduler::FrameClock;

use crate::protocol::copy_payload;
use crate::state::STATE;

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
        let scroll_rows = state
            .borrow()
            .renderer
            .previous_frame()
            .map_or(0, |frame| frame.scroll_rows);
        let payload = move_cursor_down(scroll_rows);
        copy_payload(&payload, output, capacity)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_module_line_init() {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.app = keel_core::AppState::default();
        state.clock = FrameClock::new();
        state.completion_tenths_ms = 0;
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_module_line_finish(output: *mut u8, capacity: usize) -> usize {
    if output.is_null() {
        return 0;
    }
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let scroll_rows = state
            .renderer
            .previous_frame()
            .map_or(0, |frame| frame.scroll_rows);
        let mut payload = move_cursor_down(scroll_rows);
        payload.extend(state.renderer.clear_previous().unwrap_or_default());
        state.app = keel_core::AppState::default();
        state.clock = FrameClock::new();
        state.reserved_scroll_rows = 0;
        state.stats.clears = state.stats.clears.saturating_add(1);
        state.stats.last_payload_bytes = payload.len();
        state.stats.last_rows = 0;
        copy_payload(&payload, output, capacity)
    })
}

fn move_cursor_down(rows: u16) -> Vec<u8> {
    if rows == 0 {
        return Vec::new();
    }
    format!("\x1b[{}B", rows).into_bytes()
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_module_reset() {
    STATE.with(|state| state.borrow_mut().reset());
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_module_stats() -> crate::abi::KeelNativeStats {
    STATE.with(|state| state.borrow().stats)
}
