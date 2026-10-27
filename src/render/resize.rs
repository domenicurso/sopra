use super::ansi::move_to;
use crate::input::CursorPosition;

pub(super) fn clear_rows(output: &mut Vec<u8>, origin_row: u16, rows: &[u16]) {
    for &row in rows {
        move_to(output, 0, origin_row.saturating_add(row));
        output.extend_from_slice(b"\x1b[2K");
    }
}

pub(super) fn clear_viewport(output: &mut Vec<u8>, origin_row: u16) {
    move_to(output, 0, origin_row);
    output.extend_from_slice(b"\x1b[J");
}

impl super::Renderer {
    pub(super) fn resize_origin(
        &self,
        position: CursorPosition,
        cursor_row_in_frame: u16,
        columns: u16,
    ) -> u16 {
        let columns = columns.max(1);
        let reflowed_rows = self.previous_cursor.map_or(0, |cursor| {
            if position.column == cursor.column % columns && position.column < columns - 1 {
                cursor.column / columns
            } else {
                0
            }
        });
        let prior_rows = self.previous.as_ref().map_or(0, |buffer| {
            if columns >= buffer.area.width {
                return 0;
            }
            (0..self.previous_cursor.map_or(0, |cursor| cursor.row))
                .map(|row| {
                    let used = (0..buffer.area.width)
                        .rev()
                        .find(|column| {
                            buffer
                                .cell((*column, row))
                                .is_some_and(|cell| cell.symbol() != " ")
                        })
                        .map_or(0, |column| column + 1);
                    used.saturating_sub(1) / columns
                })
                .sum::<u16>()
        });
        // Earlier prompt rows can reflow before the terminal relocates the cursor.
        // The cursor's own row may instead be clamped at the edge.

        position
            .row
            .saturating_sub(cursor_row_in_frame)
            .saturating_sub(reflowed_rows)
            .saturating_sub(prior_rows)
    }
}

#[cfg(test)]
mod tests {
    use super::clear_viewport;

    #[test]
    fn resize_clear_starts_at_the_active_prompt_and_preserves_history_above() {
        let mut output = Vec::new();

        clear_viewport(&mut output, 7);

        assert_eq!(output, b"\x1b[8;1H\x1b[J");
    }
}
