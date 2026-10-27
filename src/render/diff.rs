use ratatui::{
    buffer::{Buffer, Cell},
    layout::Rect,
};

use super::{
    Renderer,
    ansi::{cell_at, paint_cell},
};

impl Renderer {
    pub(super) fn paint_diff(
        &self,
        next: &Buffer,
        old_area: Rect,
        repaint_cells: &[(u16, u16)],
        output: &mut Vec<u8>,
    ) -> usize {
        let width = next.area.width.max(old_area.width);
        let height = next.area.height.max(old_area.height);
        let mut changed = 0;
        for row in 0..height {
            for column in 0..width {
                let next_cell = cell_at(next, column, row);
                let previous_cell = self
                    .previous
                    .as_ref()
                    .map_or(Cell::EMPTY, |frame| cell_at(frame, column, row));
                // Cleared prompt rows need no per-cell spaces; untouched rows preserve shell history.
                if self.previous.is_none() && next_cell == Cell::EMPTY {
                    continue;
                }
                if next_cell != previous_cell || repaint_cells.contains(&(column, row)) {
                    paint_cell(
                        output,
                        column,
                        self.origin_row.unwrap_or_default().saturating_add(row),
                        &next_cell,
                    );
                    changed += 1;
                }
            }
        }
        changed
    }
}
