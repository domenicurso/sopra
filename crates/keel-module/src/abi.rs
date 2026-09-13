pub(crate) const MAX_HOST_BYTES: usize = 4 * 1024 * 1024;
pub(crate) const MAX_COMPLETIONS: usize = 16_384;
pub(crate) const MAX_POPUP_ROWS: u16 = keel_ui::MAX_VISIBLE_ITEMS as u16 + 2;
pub(crate) const POPUP_MIN_ROWS: u16 = 3;
pub(crate) const COMPLETION_FIELD_SEPARATOR: u8 = 0x1f;
pub(crate) const COMPLETION_RECORD_SEPARATOR: u8 = 0x1e;
pub const KEEL_NATIVE_ABI_VERSION: u32 = 1;

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
