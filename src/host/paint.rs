use super::{browser::Browser, layout, model::Model, terminal::Terminal, view};
use ratatui::buffer::Buffer;
use std::io;

#[derive(Default)]
pub(super) struct Painter {
    previous: Option<Buffer>,
}
impl Painter {
    pub(super) fn render(
        &mut self,
        model: &Model,
        browser: Option<&Browser>,
        terminal: &mut Terminal,
    ) -> io::Result<()> {
        let buffer = view::frame(model, browser);
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
        let (mut row, column) = screen.cursor_position();
        if browser.is_some() {
            let layout = layout::Layout::new(model);
            row = layout.prompt_top
                + row
                    .saturating_sub(layout.prompt_source)
                    .min(layout.pinned.saturating_sub(1));
        }
        crate::render::move_to(&mut bytes, column, row.min(model.size.rows - 1));
        if !screen.hide_cursor() && browser.is_none() {
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
