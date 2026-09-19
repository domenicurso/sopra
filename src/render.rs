mod ansi;

pub(crate) use self::ansi::region_highlight_style;

#[cfg(test)]
mod tests;

use std::io;

use ratatui::{
    buffer::{Buffer, Cell},
    layout::Rect,
};

use crate::{
    input::{CursorPosition, Terminal},
    scene::Frame,
};

use self::ansi::{cell_at, move_to, paint_cell};

#[derive(Debug, Default)]
pub(crate) struct Renderer {
    previous: Option<Buffer>,
    cursor_hidden: bool,
    origin_saved: bool,
}

impl Renderer {
    pub(crate) fn render(&mut self, next: Frame, terminal: &mut Terminal) -> io::Result<usize> {
        let resized = self
            .previous
            .as_ref()
            .is_some_and(|previous| previous.area != next.buffer.area);
        let Frame {
            buffer: next,
            clear_rows: next_rows,
            repaint_cells,
            cursor,
        } = next;
        let old_area = self
            .previous
            .as_ref()
            .map_or(Rect::default(), |frame| *frame.area());
        let mut output = Vec::new();
        self.begin_frame(&mut output);
        if resized {
            // The terminal may reflow the active line during SIGWINCH. The frame leaves the
            // cursor there, so use its current visual row as the new scene origin before repaint.
            output.extend_from_slice(b"\r\x1b[s");
            self.previous = None;
            clear_rows(&mut output, &(0..next.area.height).collect::<Vec<_>>());
        } else if self.previous.is_none() {
            clear_rows(&mut output, &next_rows);
        }
        let changed = self.paint_diff(&next, old_area, &repaint_cells, &mut output);
        if changed > 0 || !output.is_empty() {
            move_to(&mut output, cursor.column, cursor.row);
            output.extend_from_slice(b"\x1b[0m");
            terminal.write_all(&output)?;
            terminal.flush()?;
        }
        self.previous = Some(next);
        Ok(changed)
    }

    fn begin_frame(&mut self, output: &mut Vec<u8>) {
        if !self.cursor_hidden {
            output.extend_from_slice(b"\x1b[?25l");
            self.cursor_hidden = true;
        }
        if !self.origin_saved {
            // ZLE has already placed the cursor on the active line, so this saved point is the
            // scene origin even when the shell prompt is below terminal row zero.
            output.extend_from_slice(b"\r\x1b[s");
            self.origin_saved = true;
        }
    }

    fn paint_diff(
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
                if next_cell != previous_cell || repaint_cells.contains(&(column, row)) {
                    paint_cell(output, column, row, &next_cell);
                    changed += 1;
                }
            }
        }
        changed
    }

    pub(crate) fn finish(
        &mut self,
        terminal: &mut Terminal,
        restore_position: CursorPosition,
        replacement: Option<Frame>,
    ) -> io::Result<()> {
        let Some(previous) = self.previous.take() else {
            return Ok(());
        };
        let replacement = replacement.map(|frame| frame.buffer);
        let mut output = Vec::new();
        paint_finish(&previous, replacement.as_ref(), &mut output);
        move_to(&mut output, restore_position.column, restore_position.row);
        output.extend_from_slice(b"\x1b[0m\x1b[?25h");
        terminal.write_all(&output)?;
        terminal.flush()?;
        self.cursor_hidden = false;
        self.origin_saved = false;
        Ok(())
    }
}

fn clear_rows(output: &mut Vec<u8>, rows: &[u16]) {
    for &row in rows {
        move_to(output, 0, row);
        output.extend_from_slice(b"\x1b[2K");
    }
}

fn paint_finish(previous: &Buffer, replacement: Option<&Buffer>, output: &mut Vec<u8>) {
    let width = previous
        .area
        .width
        .max(replacement.map_or(0, |frame| frame.area.width));
    let height = previous
        .area
        .height
        .max(replacement.map_or(0, |frame| frame.area.height));
    for row in 0..height {
        for column in 0..width {
            let previous_cell = cell_at(previous, column, row);
            let next_cell = replacement.map_or(Cell::EMPTY, |frame| cell_at(frame, column, row));
            if next_cell != previous_cell {
                paint_replacement_cell(output, column, row, &next_cell);
            }
        }
    }
}

fn paint_replacement_cell(output: &mut Vec<u8>, column: u16, row: u16, cell: &Cell) {
    if cell == &Cell::EMPTY {
        move_to(output, column, row);
        output.extend_from_slice(b"\x1b[0m ");
    } else {
        paint_cell(output, column, row, cell);
    }
}
