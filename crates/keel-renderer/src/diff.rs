use ratatui::{buffer::Cell, layout::Rect};

use crate::model::{FrameDiff, RenderedFrame};
use crate::renderer::Renderer;

impl Renderer {
    pub fn diff(&self, next: Option<&RenderedFrame>) -> FrameDiff {
        let previous = self.previous.as_ref();
        let previous_area = previous.map_or(Rect::new(0, 0, 0, 0), |frame| frame.area);
        let next_area = next.map_or(Rect::new(0, 0, 0, 0), |frame| frame.area);
        let previous_origin = previous.map_or(0, |frame| frame.origin_column);
        let next_origin = next.map_or(0, |frame| frame.origin_column);
        let previous_cursor_row = previous.map_or(0, |frame| frame.cursor_row);
        let next_cursor_row = next.map_or(0, |frame| frame.cursor_row);
        let previous_anchor_row = previous.map_or(0, |frame| frame.anchor_row);
        let next_anchor_row = next.map_or(0, |frame| frame.anchor_row);
        let previous_row_offset = previous.map_or(0, |frame| frame.row_offset);
        let next_row_offset = next.map_or(0, |frame| frame.row_offset);
        let width = previous_area.width.max(next_area.width);
        let height = previous_area.height.max(next_area.height);
        let mut changed_cells = 0;
        let mut changed_rows = Vec::new();
        let mut cleared_rows = Vec::new();
        let empty = Cell::EMPTY;

        for y in 0..height {
            let mut row_changed = false;
            let mut row_cleared = false;
            for x in 0..width {
                let previous_cell = previous
                    .and_then(|frame| frame.buffer.cell((x, y)))
                    .unwrap_or(&empty);
                let next_cell = next
                    .and_then(|frame| frame.buffer.cell((x, y)))
                    .unwrap_or(&empty);
                if previous_cell != next_cell {
                    changed_cells += 1;
                    row_changed = true;
                    if next.is_none() || y >= next_area.height || x >= next_area.width {
                        row_cleared = true;
                    }
                }
            }
            if row_changed {
                changed_rows.push(y);
            }
            if row_cleared {
                cleared_rows.push(y);
            }
        }

        FrameDiff {
            changed_cells,
            changed_rows,
            cleared_rows,
            previous_area,
            next_area,
            previous_origin,
            next_origin,
            previous_cursor_row,
            next_cursor_row,
            previous_anchor_row,
            next_anchor_row,
            previous_row_offset,
            next_row_offset,
        }
    }
}
