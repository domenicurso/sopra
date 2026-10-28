use std::time::Instant;

use super::EditorState;
use crate::input::{MouseEvent, MouseKind};

impl EditorState {
    pub(super) fn handle_mouse(&mut self, event: MouseEvent, origin: Option<u16>) {
        if self.widgets.active()
            || (event.kind == MouseKind::Drag && !self.text_selection.dragging())
        {
            return;
        }
        let Some(origin) = origin else { return };
        let start = self.restore_prompt_position(self.last_size, self.prompt());
        let target_row = event.position.row.saturating_sub(origin);
        if event.kind == MouseKind::Press
            && (event.position.row < origin
                || target_row < start.row
                || (target_row == start.row && event.position.column < start.column))
        {
            return;
        }
        let offset = self.offset_at(crate::input::CursorPosition {
            row: target_row,
            column: event.position.column,
        });
        let cursor = self
            .text_selection
            .mouse(event.kind, offset, &self.buffer, Instant::now());
        if event.kind != MouseKind::Release {
            self.cursor = cursor;
            self.disarm_completion();
            self.note_activity();
            self.request_completion();
        }
    }
}
