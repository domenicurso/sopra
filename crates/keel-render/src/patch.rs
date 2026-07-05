use keel_core::{CanvasBuffer, CanvasRow, TerminalSize, VisualCursor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferRowPatch {
    pub row: u16,
    pub cells: CanvasRow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasPatch {
    pub previous_size: TerminalSize,
    pub next_size: TerminalSize,
    pub full_redraw: bool,
    pub rows: Vec<BufferRowPatch>,
    pub cursor: Option<VisualCursor>,
}

impl CanvasPatch {
    pub fn between(previous: Option<&CanvasBuffer>, next: &CanvasBuffer) -> Self {
        let previous_size = previous.map(|buffer| buffer.size).unwrap_or_default();
        let full_redraw = previous.is_none() || previous_size != next.size;
        let mut rows = Vec::new();

        if full_redraw {
            rows.extend(next.rows.iter().enumerate().map(|(index, row)| BufferRowPatch {
                row: index as u16,
                cells: row.clone(),
            }));
        } else if let Some(previous) = previous {
            for (index, next_row) in next.rows.iter().enumerate() {
                if previous.rows.get(index) != Some(next_row) {
                    rows.push(BufferRowPatch {
                        row: index as u16,
                        cells: next_row.clone(),
                    });
                }
            }

            if previous.cursor != next.cursor {
                push_cursor_row(&mut rows, previous.cursor, next);
                push_cursor_row(&mut rows, next.cursor, next);
            }
        }

        Self {
            previous_size,
            next_size: next.size,
            full_redraw,
            rows,
            cursor: next.cursor,
        }
    }
}

fn push_cursor_row(rows: &mut Vec<BufferRowPatch>, cursor: Option<keel_core::VisualCursor>, next: &CanvasBuffer) {
    let Some(cursor) = cursor else {
        return;
    };
    let row_index = cursor.row as usize;
    if row_index >= next.rows.len() || rows.iter().any(|row| row.row == cursor.row) {
        return;
    }

    rows.push(BufferRowPatch {
        row: cursor.row,
        cells: next.rows[row_index].clone(),
    });
}

#[cfg(test)]
mod tests {
    use super::CanvasPatch;
    use keel_core::{CanvasBuffer, CellStyle, CursorStyle, TerminalSize, VisualCursor};

    #[test]
    fn diffs_only_changed_rows() {
        let size = TerminalSize {
            columns: 8,
            rows: 4,
        };
        let mut previous = CanvasBuffer::blank(size);
        previous.rows[0].cells[0].symbol = "a".to_string();
        let mut next = previous.clone();
        next.rows[1].cells[0].symbol = "b".to_string();
        next.rows[1].cells[0].style = CellStyle::Prompt;

        let patch = CanvasPatch::between(Some(&previous), &next);
        assert!(!patch.full_redraw);
        assert_eq!(patch.rows.len(), 1);
        assert_eq!(patch.rows[0].row, 1);
    }

    #[test]
    fn diffs_cursor_only_changes_by_invalidating_cursor_rows() {
        let size = TerminalSize {
            columns: 8,
            rows: 4,
        };
        let mut previous = CanvasBuffer::blank(size);
        previous.cursor = Some(VisualCursor {
            row: 0,
            column: 1,
            style: CursorStyle::Block,
            visible: true,
            alpha: 255,
        });

        let mut next = previous.clone();
        next.cursor = Some(VisualCursor {
            row: 0,
            column: 2,
            style: CursorStyle::Block,
            visible: true,
            alpha: 255,
        });

        let patch = CanvasPatch::between(Some(&previous), &next);
        assert_eq!(patch.rows.len(), 1);
        assert_eq!(patch.rows[0].row, 0);
    }
}
