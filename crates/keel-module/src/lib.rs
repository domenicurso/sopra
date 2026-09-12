use std::{cell::RefCell, ptr, slice};

use keel_core::{AppState, EditorEvent, HostLine, HostSnapshot, ScreenPoint, TerminalSize};
use keel_renderer::{RenderContext, Renderer};
use keel_scheduler::{FrameClock, InvalidationReason};
use keel_ui::{Constraints, PopupItem, Scene, SuggestionPopup, Theme};

const MAX_HOST_BYTES: usize = 256 * 1024;

#[repr(C)]
pub struct KeelNativeHostSnapshot {
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

struct ModuleState {
    app: AppState,
    renderer: Renderer,
    clock: FrameClock,
    theme: Theme,
    stats: KeelNativeStats,
}

impl ModuleState {
    fn new() -> Self {
        Self {
            app: AppState::default(),
            renderer: Renderer::new(),
            clock: FrameClock::new(),
            theme: Theme::default(),
            stats: KeelNativeStats::default(),
        }
    }

    fn reset(&mut self) {
        *self = Self::new();
    }
}

thread_local! {
    static STATE: RefCell<ModuleState> = RefCell::new(ModuleState::new());
}

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
        let mut state = state.borrow_mut();
        let payload = state.renderer.clear_previous().unwrap_or_default();
        state.stats.clears = state.stats.clears.saturating_add(1);
        state.stats.last_payload_bytes = payload.len();
        copy_payload(&payload, output, capacity)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_module_after_redraw(
    snapshot: *const KeelNativeHostSnapshot,
    output: *mut u8,
    capacity: usize,
) -> usize {
    if snapshot.is_null() || output.is_null() {
        return 0;
    }

    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(snapshot) = read_snapshot(snapshot) else {
            return 0;
        };
        state
            .clock
            .invalidate(InvalidationReason::HostRedisplay, std::time::Instant::now());
        state.app.apply(EditorEvent::Redisplay(snapshot.clone()));

        let Some(scene) = build_scene(&state.app, state.theme) else {
            state.clock.rendered(std::time::Instant::now());
            state.stats.last_payload_bytes = 0;
            state.stats.last_rows = 0;
            return 0;
        };

        let constraints = Constraints::new(
            snapshot.terminal.columns.saturating_sub(1).max(1),
            snapshot.terminal.rows.saturating_sub(1).clamp(1, 8),
        );
        let desired = scene.measure(constraints);
        let origin = snapshot.cursor.column.min(
            snapshot
                .terminal
                .columns
                .saturating_sub(desired.width.max(1)),
        );
        let context = RenderContext::new(snapshot.terminal.columns, constraints.max_height, origin)
            .full_repaint(true);
        let rendered = match state.renderer.render(&scene, context) {
            Ok(rendered) => rendered,
            Err(_) => return 0,
        };
        let payload = &rendered.transaction.payload;
        state.stats.redraws = state.stats.redraws.saturating_add(1);
        state.stats.last_payload_bytes = payload.len();
        state.stats.last_rows = rendered.frame.area.height;
        state.clock.rendered(std::time::Instant::now());
        copy_payload(payload, output, capacity)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_module_line_init() {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.app = AppState::default();
        state.clock = FrameClock::new();
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
        state.app = AppState::default();
        state.clock = FrameClock::new();
        state.stats.clears = state.stats.clears.saturating_add(1);
        state.stats.last_payload_bytes = payload.len();
        state.stats.last_rows = 0;
        copy_payload(&payload, output, capacity)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_module_reset() {
    STATE.with(|state| state.borrow_mut().reset());
}

#[unsafe(no_mangle)]
pub extern "C" fn keel_module_stats() -> KeelNativeStats {
    STATE.with(|state| state.borrow().stats)
}

fn build_scene(app: &AppState, theme: Theme) -> Option<Scene> {
    if app.suggestions.is_empty() {
        return None;
    }
    let items = app
        .suggestions
        .iter()
        .map(|suggestion| PopupItem::new(&suggestion.label, &suggestion.detail))
        .collect::<Vec<_>>();
    let footer = format!(
        "{}/{} · Up/Down to select",
        app.selected_suggestion.saturating_add(1),
        items.len()
    );
    Some(Scene::new(SuggestionPopup::new(
        "Keel suggestions",
        footer,
        items,
        app.selected_suggestion,
        theme,
    )))
}

fn read_snapshot(raw: *const KeelNativeHostSnapshot) -> Option<HostSnapshot> {
    // The C shim owns these buffers for the duration of the call.
    let raw = unsafe { &*raw };
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
    let bytes = unsafe { slice::from_raw_parts(pointer, length) };
    Some(String::from_utf8_lossy(bytes).into_owned())
}

fn copy_payload(payload: &[u8], output: *mut u8, capacity: usize) -> usize {
    if payload.len() > capacity {
        return 0;
    }
    unsafe {
        ptr::copy_nonoverlapping(payload.as_ptr(), output, payload.len());
    }
    payload.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(text: &[u8], cursor_column: u16) -> KeelNativeHostSnapshot {
        KeelNativeHostSnapshot {
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

    #[test]
    fn empty_input_has_no_overlay() {
        keel_module_init();
        let mut output = [0_u8; 4096];
        let raw = snapshot(b"", 0);
        assert_eq!(
            keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len()),
            0
        );
    }

    #[test]
    fn non_empty_input_renders_the_popup() {
        keel_module_init();
        let mut output = [0_u8; 16 * 1024];
        let raw = snapshot(b"grep --matches", 4);
        let size = keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len());
        assert!(size > 0);
        let rendered = String::from_utf8_lossy(&output[..size]);
        assert!(rendered.contains("--files-with-matches"));
        assert!(rendered.contains("--files-without-match"));
    }

    #[test]
    fn pre_redraw_clears_the_previous_surface() {
        keel_module_init();
        let mut output = [0_u8; 16 * 1024];
        let raw = snapshot(b"git", 3);
        assert!(keel_module_after_redraw(&raw, output.as_mut_ptr(), output.len()) > 0);
        let size = keel_module_before_redraw(output.as_mut_ptr(), output.len());
        assert!(size > 0);
        assert!(String::from_utf8_lossy(&output[..size]).contains("\x1b["));
    }
}
