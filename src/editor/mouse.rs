use std::time::Instant;

use super::EditorState;
use crate::input::{MouseEvent, MouseKind};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

impl EditorState {
    pub(super) fn handle_mouse(&mut self, event: MouseEvent, origin: Option<u16>) {
        if self.widgets.active()
            || (event.kind == MouseKind::Drag && !self.text_selection.dragging())
        {
            return;
        }
        let Some(origin) = origin else { return };
        let start = self.restore_prompt_position(self.last_size, self.prompt());
        let mut flow = crate::scene::text_flow::TextFlow::new(start, self.last_size.columns);
        let target_row = event.position.row.saturating_sub(origin);
        if event.kind == MouseKind::Press
            && (event.position.row < origin
                || target_row < start.row
                || (target_row == start.row && event.position.column < start.column))
        {
            return;
        }
        let mut offset = self.buffer.len();
        for (byte, grapheme) in self.buffer.grapheme_indices(true) {
            let Some(position) = flow.advance(grapheme) else {
                continue;
            };
            if target_row < position.row
                || (target_row == position.row
                    && event.position.column
                        < position.column.saturating_add(grapheme.width() as u16))
            {
                offset = byte;
                break;
            }
        }
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
