use std::time::Instant;

use super::{EditorState, text_selection::offset_for_column};
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
        let row = origin.saturating_add(start.row);
        if event.kind == MouseKind::Press && event.position.row != row {
            return;
        }
        if event.kind == MouseKind::Press && event.position.column < start.column {
            return;
        }
        let offset = if event.position.row < row {
            0
        } else if event.position.row > row {
            self.buffer.len()
        } else {
            offset_for_column(
                &self.buffer,
                event.position.column.saturating_sub(start.column) as usize,
            )
        };
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
