use std::time::Instant;

use keel_core::EditorEvent;
use keel_renderer::RenderContext;
use keel_scheduler::InvalidationReason;
use keel_ui::{Constraints, PopupPlacement};

use crate::abi::{KeelNativeHostSnapshot, MAX_POPUP_ROWS, POPUP_MIN_ROWS};
use crate::protocol::{copy_payload, read_snapshot};
use crate::render::{build_scene, choose_popup_placement, popup_geometry};
use crate::state::STATE;

#[unsafe(no_mangle)]
pub extern "C" fn keel_module_observe(snapshot: *const KeelNativeHostSnapshot) -> i32 {
    if snapshot.is_null() {
        return 0;
    }

    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(snapshot) = read_snapshot(snapshot) else {
            return 0;
        };
        state.app.apply(EditorEvent::Redisplay(snapshot));
        state.app.refresh_suggestions();
        1
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
        let now = Instant::now();
        state
            .clock
            .invalidate(InvalidationReason::HostRedisplay, now);
        state.app.apply(EditorEvent::Redisplay(snapshot.clone()));
        state.app.refresh_suggestions();
        let completion_tenths_ms = state.completion_tenths_ms;

        let Some(probe) = build_scene(&state.app, 0, PopupPlacement::Below, completion_tenths_ms)
        else {
            let payload = state
                .renderer
                .clear_previous_at(snapshot.cursor.row)
                .unwrap_or_default();
            state.clock.rendered(Instant::now());
            state.stats.last_payload_bytes = payload.len();
            state.stats.last_rows = 0;
            return copy_payload(&payload, output, capacity);
        };
        let columns = snapshot.terminal.columns.max(1);
        let natural = probe.measure(Constraints::new(columns, u16::MAX));
        let requested_height = natural.height.clamp(POPUP_MIN_ROWS, MAX_POPUP_ROWS);
        let below = snapshot
            .terminal
            .rows
            .saturating_sub(snapshot.cursor.row.saturating_add(1));
        let above = snapshot.cursor.row;
        let Some((placement, height)) = choose_popup_placement(requested_height, below, above)
        else {
            let payload = state
                .renderer
                .clear_previous_at(snapshot.cursor.row)
                .unwrap_or_default();
            state.clock.rendered(Instant::now());
            state.stats.last_payload_bytes = payload.len();
            state.stats.last_rows = 0;
            return copy_payload(&payload, output, capacity);
        };

        let width = natural.width.min(columns).max(1);
        let (origin, anchor) = popup_geometry(
            snapshot.cursor.column,
            state.app.completion_token_width(),
            width,
            columns,
        );
        let Some(scene) = build_scene(&state.app, anchor, placement, completion_tenths_ms) else {
            return 0;
        };
        let row_offset = match placement {
            PopupPlacement::Below => 1,
            PopupPlacement::Above => -(height as i16),
        };
        let context = RenderContext::new(columns, height, origin)
            .cursor_row(snapshot.cursor.row)
            .row_offset(row_offset);
        let rendered = match state.renderer.render(&scene, context) {
            Ok(rendered) => rendered,
            Err(_) => return 0,
        };
        let payload = &rendered.transaction.payload;
        state.stats.redraws = state.stats.redraws.saturating_add(1);
        state.stats.last_payload_bytes = payload.len();
        state.stats.last_rows = rendered.frame.area.height;
        state.clock.rendered(Instant::now());
        copy_payload(payload, output, capacity)
    })
}
