use std::ffi::c_char;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeelStatusCode {
    Ok = 0,
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NotActive = 3,
    InvalidUtf8 = 4,
    InvalidRequest = 5,
    Cancelled = 6,
    IoError = 7,
    Panic = 8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct KeelCommandReadRequest {
    pub prompt: *const c_char,
    pub initial_buffer: *const c_char,
    pub initial_cursor: usize,
}
