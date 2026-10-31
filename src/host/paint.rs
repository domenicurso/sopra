use super::{cells, model::Model, terminal::Terminal};
use ratatui::buffer::Buffer;
use std::io;

#[derive(Default)]
pub(super) struct Painter {
    previous: Option<Buffer>,
}
impl Painter {
    pub(super) fn render(&mut self, model: &Model, terminal: &mut Terminal) -> io::Result<()> {
        let mut buffer = Buffer::empty(ratatui::layout::Rect::new(
            0,
            0,
            model.size.cols,
            model.size.rows,
        ));
        cells::copy_screen(&mut buffer, model.terminal.screen());
        let mut bytes = b"\x1b[?2026h\x1b[?7l\x1b[?25l".to_vec();
        let empty = Buffer::empty(buffer.area);
        let previous = self
            .previous
            .as_ref()
            .filter(|previous| previous.area == buffer.area)
            .unwrap_or(&empty);
        if self
            .previous
            .as_ref()
            .is_none_or(|previous| previous.area != buffer.area)
        {
            bytes.extend_from_slice(b"\x1b[H\x1b[2J");
        }
        for (column, row, cell) in previous.diff(&buffer) {
            crate::render::paint_cell(&mut bytes, column, row, cell);
        }
        let screen = model.terminal.screen();
        let (row, column) = screen.cursor_position();
        crate::render::move_to(&mut bytes, column, row.min(model.size.rows - 1));
        if !screen.hide_cursor() && screen.scrollback() == 0 {
            bytes.extend_from_slice(b"\x1b[?25h");
        }
        bytes.extend_from_slice(if screen.application_cursor() {
            b"\x1b[?1h"
        } else {
            b"\x1b[?1l"
        });
        bytes.extend_from_slice(b"\x1b[0m\x1b[?7h\x1b[?2026l");
        terminal.write(&bytes)?;
        self.previous = Some(buffer);
        Ok(())
    }
}
