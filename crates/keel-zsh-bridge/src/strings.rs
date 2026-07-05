use std::ffi::{CStr, CString, c_char};

use crate::KeelStatusCode;

pub(crate) fn sanitize_cstring(message: impl Into<String>) -> CString {
    let sanitized = message.into().replace('\0', " ");
    CString::new(sanitized).unwrap_or_else(|_| CString::new("keel bridge error").unwrap())
}

pub(crate) fn decode_optional_string(
    value: *const c_char,
    default: &str,
) -> Result<String, KeelStatusCode> {
    if value.is_null() {
        return Ok(default.to_string());
    }

    unsafe { CStr::from_ptr(value) }
        .to_str()
        .map(|text| text.to_string())
        .map_err(|_| KeelStatusCode::InvalidUtf8)
}
