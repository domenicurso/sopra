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
pub extern "C" fn keel_module_before_redraw(_output: *mut u8, _capacity: usize) -> usize {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_module_line_init() {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.app = keel_core::AppState::default();
        state.clock = FrameClock::new();
        state.completion_tenths_ms = 0;
        state.last_scroll_rows = 0;
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
        state.app = keel_core::AppState::default();
        state.clock = FrameClock::new();
        state.last_scroll_rows = 0;
        state.stats.clears = state.stats.clears.saturating_add(1);
        state.stats.last_payload_bytes = payload.len();
        state.stats.last_rows = 0;
        copy_payload(&payload, output, capacity)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_module_last_scroll_rows() -> u16 {
    STATE.with(|state| state.borrow().last_scroll_rows)
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_module_reset() {
    STATE.with(|state| state.borrow_mut().reset());
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_module_stats() -> crate::abi::KeelNativeStats {
    STATE.with(|state| state.borrow().stats)
}
