use std::{
    ffi::CString,
    sync::{Mutex, OnceLock},
};

#[derive(Debug, Clone, Default)]
pub(crate) struct BridgeState {
    pub(crate) initialized: bool,
    pub(crate) active: bool,
}

pub(crate) fn bridge_state() -> &'static Mutex<BridgeState> {
    static BRIDGE_STATE: OnceLock<Mutex<BridgeState>> = OnceLock::new();
    BRIDGE_STATE.get_or_init(|| Mutex::new(BridgeState::default()))
}

fn last_error_slot() -> &'static Mutex<Option<CString>> {
    static LAST_ERROR: OnceLock<Mutex<Option<CString>>> = OnceLock::new();
    LAST_ERROR.get_or_init(|| Mutex::new(None))
}

pub(crate) fn set_last_error(message: CString) {
    if let Ok(mut slot) = last_error_slot().lock() {
        *slot = Some(message);
    }
}

pub(crate) fn clear_last_error() {
    if let Ok(mut slot) = last_error_slot().lock() {
        *slot = None;
    }
}

pub(crate) fn last_error_ptr() -> *const std::ffi::c_char {
    let guard = match last_error_slot().lock() {
        Ok(guard) => guard,
        Err(_) => return std::ptr::null(),
    };

    match guard.as_ref() {
        Some(message) => message.as_ptr(),
        None => std::ptr::null(),
    }
}
