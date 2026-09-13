use std::fmt::Write as _;

use crate::model::{FrameDiff, RenderedFrame};

use super::{AnsiWriter, style::write_row};

impl AnsiWriter {
    pub(crate) fn clear_surface(
        origin_column: u16,
        row_offset: i16,
        width: u16,
        height: u16,
        scroll_rows: u16,
    ) -> Vec<u8> {
        if width == 0 || height == 0 {
            return Vec::new();
        }
        let mut output = String::new();
        output.push_str("\x1b7\x1b[?25l");
        move_relative_rows(&mut output, row_offset);
        clear_surface_body(&mut output, origin_column, 0, width, height);
        output.push_str("\x1b[0m\x1b8");
        move_relative_rows(&mut output, -(scroll_rows.min(i16::MAX as u16) as i16));
        if scroll_rows > 0 {
            let _ = write!(output, "\x1b[{}T", scroll_rows);
            move_relative_rows(&mut output, scroll_rows.min(i16::MAX as u16) as i16);
        }
        output.push_str("\x1b[?25h");
        output.into_bytes()
    }

    pub(crate) fn paint(frame: &RenderedFrame, diff: &FrameDiff, force_full: bool) -> Vec<u8> {
        if !force_full && !diff.changed() {
            if frame.scroll_rows == 0 {
                return Vec::new();
            }
            let mut output = String::new();
            move_relative_rows(
                &mut output,
                -(frame.scroll_rows.min(i16::MAX as u16) as i16),
            );
            return output.into_bytes();
        }
        let mut output = String::new();
        output.push_str("\x1b7\x1b[?25l");
        if diff.origin_changed() && diff.previous_area.height > 0 {
            output.push_str("\x1b8");
            clear_surface_body(
                &mut output,
                diff.previous_origin,
                clear_row_offset(diff, frame.cursor_row),
                diff.previous_area.width,
                diff.previous_area.height,
            );
            output.push_str("\x1b8");
        }
        if diff.scroll_rows_added() > 0 {
            let _ = write!(output, "\x1b[{}S", diff.scroll_rows_added());
        }
        if diff.scroll_rows_removed() > 0 {
            let _ = write!(output, "\x1b[{}T", diff.scroll_rows_removed());
        }
        move_relative_rows(&mut output, frame.row_offset);
        move_to_column(&mut output, frame.origin_column);
        let clear_width = frame.area.width.max(diff.previous_area.width);
        let row_count = frame.area.height.max(diff.previous_area.height);
        let repaint_all = force_full || diff.origin_changed();
        for row in 0..row_count {
            if row < frame.area.height && (repaint_all || diff.changed_rows.contains(&row)) {
                output.push_str("\x1b[0m");
                erase_characters(&mut output, clear_width);
                write_row(&mut output, frame, row);
            } else if row >= frame.area.height && diff.cleared_rows.contains(&row) {
                output.push_str("\x1b[0m");
                erase_characters(&mut output, diff.previous_area.width);
            }
            if row + 1 < row_count {
                output.push_str("\x1b[1B");
                move_to_column(&mut output, frame.origin_column);
            }
        }
        output.push_str("\x1b[0m\x1b8");
        move_relative_rows(
            &mut output,
            -(frame.scroll_rows.min(i16::MAX as u16) as i16),
        );
        output.push_str("\x1b[?25h");
        output.into_bytes()
    }
}

fn move_relative_rows(output: &mut String, offset: i16) {
    if offset > 0 {
        let _ = write!(output, "\x1b[{}B", offset);
    } else if offset < 0 {
        let _ = write!(output, "\x1b[{}A", offset.unsigned_abs());
    }
}

fn move_to_column(output: &mut String, column: u16) {
    let _ = write!(output, "\x1b[{}G", column.saturating_add(1));
}

fn clear_surface_body(
    output: &mut String,
    origin_column: u16,
    row_offset: i16,
    width: u16,
    height: u16,
) {
    move_relative_rows(output, row_offset);
    move_to_column(output, origin_column);
    for row in 0..height {
        output.push_str("\x1b[0m");
        erase_characters(output, width);
        if row + 1 < height {
            output.push_str("\x1b[1B");
            move_to_column(output, origin_column);
        }
    }
}

fn clear_row_offset(diff: &FrameDiff, cursor_row: u16) -> i16 {
    if diff.previous_cursor_row == cursor_row {
        diff.previous_row_offset
    } else {
        (i32::from(diff.previous_anchor_row) - i32::from(cursor_row))
            .clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
    }
}

fn erase_characters(output: &mut String, width: u16) {
    let _ = write!(output, "\x1b[{}X", width);
}
