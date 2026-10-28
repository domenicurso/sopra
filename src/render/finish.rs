use std::io;

use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::Rect;

use crate::{
    input::{CursorPosition, Terminal, TerminalSize},
    scene::Frame,
};

use super::{
    Renderer,
    ansi::{cell_at, move_to, paint_cell, with_terminal_modes},
    resize::clear_viewport,
};

impl Renderer {
    pub(crate) fn finish(
        &mut self,
        terminal: &mut Terminal,
        restore_position: CursorPosition,
        replacement: Option<Frame>,
    ) -> io::Result<()> {
        let Some(previous_area) = self.previous.as_ref().map(|previous| previous.area) else {
            return Ok(());
        };
        let origin_row = self.finish_origin(previous_area, terminal);
        self.previous.take();
        let mut output = Vec::new();
        clear_viewport(&mut output, origin_row);
        if let Some(frame) = &replacement {
            paint_finish(Some(&frame.buffer), &[], origin_row, &mut output);
        }
        move_to(
            &mut output,
            restore_position.column,
            origin_row.saturating_add(restore_position.row),
        );
        output.extend_from_slice(b"\x1b[0m\x1b[?25h");
        terminal.write_all(&with_terminal_modes(&output))?;
        terminal.flush()?;
        self.cursor_hidden = false;
        self.origin_row = None;
        self.resize_epoch = None;
        Ok(())
    }

    fn finish_origin(&self, previous_area: Rect, terminal: &mut Terminal) -> u16 {
        let size = terminal
            .size()
            .unwrap_or(TerminalSize::new(previous_area.width, previous_area.height));
        let area = Rect::new(0, 0, size.columns, size.rows);
        let resized = self.resize_detected(area, terminal.resize_epoch());
        if resized {
            return terminal.cursor_position().map_or(
                self.origin_row.unwrap_or_default(),
                |position| {
                    self.resize_origin(
                        position,
                        self.previous_cursor.map_or(0, |cursor| cursor.row),
                        size.columns,
                    )
                },
            );
        }
        self.origin_row.unwrap_or_default()
    }
}

pub(super) fn paint_finish(
    replacement: Option<&Buffer>,
    clear_rows: &[u16],
    origin_row: u16,
    output: &mut Vec<u8>,
) {
    for &row in clear_rows {
        move_to(output, 0, row);
        output.extend_from_slice(b"\x1b[2K");
    }
    let Some(replacement) = replacement else {
        return;
    };
    for row in 0..replacement.area.height {
        for column in 0..replacement.area.width {
            let cell = cell_at(replacement, column, row);
            if cell != Cell::EMPTY {
                paint_cell(output, column, origin_row.saturating_add(row), &cell);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{buffer::Buffer, layout::Rect};

    use super::paint_finish;

    #[test]
    fn transient_finish_erases_the_line_without_printing_trailing_spaces() {
        let mut replacement = Buffer::empty(Rect::new(0, 0, 120, 2));
        replacement[(0, 0)].set_symbol("$");
        replacement[(2, 0)].set_symbol("e");
        let mut output = Vec::new();

        paint_finish(Some(&replacement), &[4], 3, &mut output);

        assert!(output.starts_with(b"\x1b[5;1H\x1b[2K"));
        assert!(output.contains(&b'e'));
        assert!(!output.windows(4).any(|window| window == b"    "));
        assert!(!output.ends_with(b" "));
    }
}
