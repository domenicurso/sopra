use unicode_segmentation::UnicodeSegmentation;

use super::EditorState;
use crate::{input::CursorPosition, scene::text_flow::TextFlow};

impl EditorState {
    pub(super) fn buffer_positions(&self) -> Vec<(usize, CursorPosition)> {
        let start = self.restore_prompt_position(self.last_size, self.prompt());
        let mut flow = TextFlow::new(start, self.last_size.columns);
        let secondary = crate::prompt::parse(self.secondary_prompt());
        let mut positions = Vec::new();
        for (byte, grapheme) in self.buffer.grapheme_indices(true) {
            if grapheme == "\n" {
                positions.push((byte, flow.cursor()));
                flow.advance(grapheme);
                flow.prompt(&secondary);
            } else if let Some(position) = flow.advance(grapheme) {
                positions.push((byte, position));
            }
        }
        positions.push((self.buffer.len(), flow.cursor()));
        positions
    }

    pub(super) fn offset_at(&self, target: CursorPosition) -> usize {
        let positions = self.buffer_positions();
        let mut on_row = positions
            .iter()
            .filter(|(_, position)| position.row == target.row);
        let first = on_row.clone().next();
        on_row
            .rfind(|(_, position)| position.column <= target.column)
            .or(first)
            .map_or_else(
                || {
                    if positions
                        .first()
                        .is_some_and(|(_, start)| target.row < start.row)
                    {
                        0
                    } else {
                        self.buffer.len()
                    }
                },
                |(byte, _)| *byte,
            )
    }

    pub(super) fn move_vertical(&mut self, direction: i8) -> bool {
        let current = self.restore_position(self.last_size);
        let Some(row) = current.row.checked_add_signed(i16::from(direction)) else {
            return false;
        };
        let positions = self.buffer_positions();
        let Some((offset, _)) = positions
            .iter()
            .filter(|(_, position)| position.row == row)
            .min_by_key(|(_, position)| position.column.abs_diff(current.column))
        else {
            return false;
        };
        self.cursor = *offset;
        self.text_selection.clear();
        true
    }
}
