mod state;
mod strings;
mod types;

use std::{
    ffi::c_char,
    panic::{AssertUnwindSafe, catch_unwind},
    ptr,
};

use keel_core::{PromptConfig, PromptToken, RuntimeOutcome, SessionConfig};
use keel_shell::ShellContextCollector;
use keel_terminal::{CrosstermTerminal, run_command_read_session};
use state::{bridge_state, clear_last_error, last_error_ptr, set_last_error};
use strings::{decode_optional_string, sanitize_cstring};

pub use types::{KeelCommandReadRequest, KeelStatusCode};

fn set_status(status_out: *mut KeelStatusCode, status: KeelStatusCode) {
    if status_out.is_null() {
        return;
    }

    unsafe {
        *status_out = status;
    }
}

fn ensure_ready_for_command_read() -> Result<(), KeelStatusCode> {
    let guard = bridge_state().lock().map_err(|_| KeelStatusCode::Panic)?;

    if !guard.initialized {
        return Err(KeelStatusCode::NotInitialized);
    }

    if !guard.active {
        return Err(KeelStatusCode::NotActive);
    }

    Ok(())
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_bridge_module_init() -> KeelStatusCode {
    let mut guard = match bridge_state().lock() {
        Ok(guard) => guard,
        Err(_) => {
            set_last_error(sanitize_cstring("bridge state lock poisoned during module init"));
            return KeelStatusCode::Panic;
        }
    };

    if guard.initialized {
        set_last_error(sanitize_cstring("bridge already initialized"));
        return KeelStatusCode::AlreadyInitialized;
    }

    guard.initialized = true;
    guard.active = false;
    clear_last_error();
    KeelStatusCode::Ok
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_bridge_module_shutdown() {
    if let Ok(mut guard) = bridge_state().lock() {
        guard.initialized = false;
        guard.active = false;
    }

    clear_last_error();
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_bridge_activate_editor() -> KeelStatusCode {
    let mut guard = match bridge_state().lock() {
        Ok(guard) => guard,
        Err(_) => {
            set_last_error(sanitize_cstring("bridge state lock poisoned during activation"));
            return KeelStatusCode::Panic;
        }
    };

    if !guard.initialized {
        set_last_error(sanitize_cstring("bridge not initialized"));
        return KeelStatusCode::NotInitialized;
    }

    guard.active = true;
    clear_last_error();
    KeelStatusCode::Ok
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_bridge_deactivate_editor() {
    if let Ok(mut guard) = bridge_state().lock() {
        guard.active = false;
    }

    clear_last_error();
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_bridge_read_command(
    request: *const KeelCommandReadRequest,
    status_out: *mut KeelStatusCode,
) -> *mut c_char {
    set_status(status_out, KeelStatusCode::Panic);

    if request.is_null() {
        set_last_error(sanitize_cstring("command-read request pointer was null"));
        set_status(status_out, KeelStatusCode::InvalidRequest);
        return ptr::null_mut();
    }

    if let Err(status) = ensure_ready_for_command_read() {
        set_last_error(sanitize_cstring(format!(
            "bridge not ready for command read: {status:?}"
        )));
        set_status(status_out, status);
        return ptr::null_mut();
    }

    let request = unsafe { &*request };
    let prompt = match decode_optional_string(request.prompt, "keel> ") {
        Ok(prompt) => prompt,
        Err(status) => {
            set_last_error(sanitize_cstring("prompt was not valid utf-8"));
            set_status(status_out, status);
            return ptr::null_mut();
        }
    };
    let initial_buffer = match decode_optional_string(request.initial_buffer, "") {
        Ok(buffer) => buffer,
        Err(status) => {
            set_last_error(sanitize_cstring("initial buffer was not valid utf-8"));
            set_status(status_out, status);
            return ptr::null_mut();
        }
    };

    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let transient_prompt = "> ".to_string();
        let mut terminal = CrosstermTerminal::enter()?;
        run_command_read_session(
            &mut terminal,
            SessionConfig {
                prompt: PromptConfig {
                    active_left: vec![PromptToken::Literal(prompt.clone())],
                    active_right: Vec::new(),
                    transient_left: vec![PromptToken::Literal(transient_prompt)],
                },
                initial_buffer,
                initial_cursor: request.initial_cursor,
                shell: ShellContextCollector::capture(),
                ..SessionConfig::default()
            },
        )
    }));

    match outcome {
        Ok(Ok(RuntimeOutcome::Accepted(command))) => {
            clear_last_error();
            set_status(status_out, KeelStatusCode::Ok);
            sanitize_cstring(command).into_raw()
        }
        Ok(Ok(RuntimeOutcome::Cancelled)) => {
            clear_last_error();
            set_status(status_out, KeelStatusCode::Cancelled);
            ptr::null_mut()
        }
        Ok(Err(error)) => {
            set_last_error(sanitize_cstring(format!(
                "interactive command read failed: {error}"
            )));
            set_status(status_out, KeelStatusCode::IoError);
            ptr::null_mut()
        }
        Err(_) => {
            set_last_error(sanitize_cstring("panic crossed the zsh bridge boundary"));
            set_status(status_out, KeelStatusCode::Panic);
            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_bridge_last_error_message() -> *const c_char {
    last_error_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_bridge_free_string(value: *mut c_char) {
    if value.is_null() {
        return;
    }

    unsafe {
        drop(std::ffi::CString::from_raw(value));
    }
}
