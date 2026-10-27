mod ansi;
mod diff;
mod finish;
mod resize;

pub(crate) use self::ansi::region_highlight_style;

#[cfg(test)]
mod tests;

use std::io;

use ratatui::{buffer::Buffer, layout::Rect};

use crate::{
    input::{CursorPosition, Terminal},
    scene::Frame,
};

use self::ansi::{mark_prompt_start, move_to, with_terminal_modes};
use self::resize::{clear_rows, clear_viewport};

#[derive(Debug, Default)]
pub(crate) struct Renderer {
    previous: Option<Buffer>,
    cursor_hidden: bool,
    origin_row: Option<u16>,
    preserve_background: bool,
    resize_epoch: Option<usize>,
    redraw_prompt: bool,
    previous_cursor: Option<CursorPosition>,
}

impl Renderer {
    pub(crate) fn overlay() -> Self {
        Self {
            preserve_background: true,
            origin_row: Some(0),
            ..Self::default()
        }
    }

    pub(crate) fn editor() -> Self {
        Self {
            redraw_prompt: true,
            ..Self::default()
        }
    }

    pub(crate) fn render(&mut self, next: Frame, terminal: &mut Terminal) -> io::Result<usize> {
        let resize_epoch = terminal.resize_epoch();
        let resized = self.resize_detected(next.buffer.area, resize_epoch);
        let old_area = self
            .previous
            .as_ref()
            .map_or(Rect::default(), |frame| *frame.area());
        let mut output = Vec::new();
        let parked = self.terminal_cursor(next.cursor);
        let origin = self.position_origin(&next, resized, terminal, parked);
        let size = terminal.size()?;
        if terminal.resize_epoch() != resize_epoch
            || size.columns != next.buffer.area.width
            || size.rows != next.buffer.area.height
        {
            return Ok(0);
        }
        self.origin_row = origin;
        if resized {
            self.previous = None;
        }
        self.clear_before_paint(&next, resized, &mut output);
        let mark_prompt = self.redraw_prompt && (resized || self.previous.is_none());
        if mark_prompt {
            mark_prompt_start(&mut output, self.origin_row.unwrap_or_default());
        }
        let changed = self.paint_diff(&next.buffer, old_area, &next.repaint_cells, &mut output);
        if changed > 0 || !output.is_empty() {
            move_to(
                &mut output,
                parked.column,
                self.origin_row
                    .unwrap_or_default()
                    .saturating_add(parked.row),
            );
            if mark_prompt {
                output.extend_from_slice(b"\x1b]133;B\x07");
            }
            output.extend_from_slice(b"\x1b[0m");
            self.write_frame(output, terminal)?;
        }
        self.previous = Some(next.buffer);
        self.previous_cursor = Some(parked);
        self.resize_epoch = Some(resize_epoch);
        Ok(changed)
    }

    fn terminal_cursor(&self, cursor: CursorPosition) -> CursorPosition {
        // Keeping the hidden cursor on the first row prevents reflow into scrollback.
        if self.redraw_prompt && cursor.row > 0 {
            CursorPosition { row: 0, column: 0 }
        } else {
            cursor
        }
    }

    fn position_origin(
        &self,
        next: &Frame,
        resized: bool,
        terminal: &mut Terminal,
        parked: CursorPosition,
    ) -> Option<u16> {
        let origin = if resized && self.preserve_background {
            Some(0)
        } else if resized {
            Some(
                terminal
                    .cursor_position()
                    .map_or(self.origin_row.unwrap_or_default(), |position| {
                        self.resize_origin(position, parked.row, next.buffer.area.width)
                    }),
            )
        } else if self.origin_row.is_none() {
            terminal.cursor_position().map(|position| position.row)
        } else {
            self.origin_row
        };
        origin.or((next.required_height > 1).then_some(0))
    }

    fn clear_before_paint(&mut self, next: &Frame, resized: bool, output: &mut Vec<u8>) {
        self.ensure_space(next.required_height, next.buffer.area.height, output);
        if resized && !self.preserve_background {
            // The editor owns the viewport from its prompt down, including reflowed rows missed
            // between frames; clearing it leaves the shell history above the prompt untouched.
            clear_viewport(output, self.origin_row.unwrap_or_default());
        } else if self.previous.is_none() {
            clear_rows(
                output,
                self.origin_row.unwrap_or_default(),
                &next.clear_rows,
            );
        }
    }

    fn write_frame(&mut self, output: Vec<u8>, terminal: &mut Terminal) -> io::Result<()> {
        let mut frame = Vec::with_capacity(output.len().saturating_add(32));
        if !self.cursor_hidden {
            frame.extend_from_slice(b"\x1b[?25l");
        }
        frame.extend_from_slice(&output);
        terminal.write_all(&with_terminal_modes(&frame))?;
        terminal.flush()?;
        self.cursor_hidden = true;
        Ok(())
    }

    fn resize_detected(&self, next_area: Rect, resize_epoch: usize) -> bool {
        self.previous
            .as_ref()
            .is_some_and(|previous| previous.area != next_area)
            || self
                .resize_epoch
                .is_some_and(|previous| previous != resize_epoch)
    }

    fn ensure_space(&mut self, required_height: u16, terminal_rows: u16, output: &mut Vec<u8>) {
        let Some(origin_row) = self.origin_row else {
            return;
        };
        let rows = rows_to_scroll(origin_row, required_height, terminal_rows);
        if rows == 0 {
            return;
        }
        move_to(output, 0, terminal_rows.saturating_sub(1));
        for _ in 0..rows {
            output.extend_from_slice(b"\r\n");
        }
        self.origin_row = Some(origin_row - rows);
        self.previous = None;
    }
}

fn rows_to_scroll(origin_row: u16, required_height: u16, terminal_rows: u16) -> u16 {
    origin_row
        .saturating_add(required_height)
        .saturating_sub(terminal_rows)
        .min(origin_row)
}
