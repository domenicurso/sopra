use super::EditorState;
use crate::input::CursorPosition;

impl EditorState {
    pub(crate) fn text_layout(
        &self,
        size: crate::input::TerminalSize,
    ) -> std::rc::Rc<crate::text::layout::Layout> {
        self.layout.get(crate::text::layout::Input {
            anchor: self.anchor,
            columns: size.columns,
            prompt: self.prompt(),
            secondary: &crate::prompt::parse(self.secondary_prompt()),
            buffer: self.buffer(),
        })
    }

    pub(super) fn buffer_positions(&self) -> Vec<(usize, CursorPosition)> {
        self.text_layout(self.last_size).positions.clone()
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
