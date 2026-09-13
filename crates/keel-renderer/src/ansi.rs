use std::fmt::Write as _;

use ratatui::{
    buffer::Cell,
    style::{Color, Modifier, Style},
};
use unicode_width::UnicodeWidthStr;

use crate::model::{FrameDiff, RenderedFrame};

pub(crate) struct AnsiWriter;

impl AnsiWriter {
    pub(crate) fn clear_surface(
        origin_column: u16,
        row_offset: i16,
        width: u16,
        height: u16,
    ) -> Vec<u8> {
        if width == 0 || height == 0 {
            return Vec::new();
        }
        let mut output = String::new();
        output.push_str("\x1b7\x1b[?25l");
        move_relative_rows(&mut output, row_offset);
        move_to_column(&mut output, origin_column);
        for row in 0..height {
            output.push_str("\x1b[0m");
            erase_characters(&mut output, width);
            if row + 1 < height {
                output.push_str("\x1b[1B");
                move_to_column(&mut output, origin_column);
            }
        }
        output.push_str("\x1b[0m\x1b8\x1b[?25h");
        output.into_bytes()
    }

    pub(crate) fn paint(frame: &RenderedFrame, diff: &FrameDiff, force_full: bool) -> Vec<u8> {
        if !force_full && !diff.changed() {
            return Vec::new();
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
        output.push_str("\x1b[0m\x1b8\x1b[?25h");
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

fn write_row(output: &mut String, frame: &RenderedFrame, row: u16) {
    let mut active_style = Style::default();
    let mut previous_wide = false;
    let empty = Cell::EMPTY;
    for x in 0..frame.area.width {
        let cell = frame.buffer.cell((x, row)).unwrap_or(&empty);
        if previous_wide {
            previous_wide = false;
            continue;
        }
        set_style(output, &mut active_style, cell.style());
        output.push_str(cell.symbol());
        previous_wide = cell.symbol().width() > 1;
    }
    output.push_str("\x1b[0m");
}

fn set_style(output: &mut String, current: &mut Style, next: Style) {
    if *current == next {
        return;
    }
    output.push_str("\x1b[0m");
    if let Some(color) = next.fg {
        write_color(output, color, false);
    }
    if let Some(color) = next.bg {
        write_color(output, color, true);
    }
    let modifier_codes = [
        (Modifier::BOLD, "1"),
        (Modifier::DIM, "2"),
        (Modifier::ITALIC, "3"),
        (Modifier::UNDERLINED, "4"),
        (Modifier::SLOW_BLINK, "5"),
        (Modifier::RAPID_BLINK, "6"),
        (Modifier::REVERSED, "7"),
        (Modifier::HIDDEN, "8"),
        (Modifier::CROSSED_OUT, "9"),
    ];
    for (modifier, code) in modifier_codes {
        if next.add_modifier.contains(modifier) {
            let _ = write!(output, "\x1b[{}m", code);
        }
    }
    *current = next;
}

fn write_color(output: &mut String, color: Color, background: bool) {
    let prefix = if background { 48 } else { 38 };
    match color {
        Color::Reset => {
            let _ = write!(output, "\x1b[{}m", if background { 49 } else { 39 });
        }
        Color::Black => output.push_str(if background { "\x1b[40m" } else { "\x1b[30m" }),
        Color::Red => output.push_str(if background { "\x1b[41m" } else { "\x1b[31m" }),
        Color::Green => output.push_str(if background { "\x1b[42m" } else { "\x1b[32m" }),
        Color::Yellow => output.push_str(if background { "\x1b[43m" } else { "\x1b[33m" }),
        Color::Blue => output.push_str(if background { "\x1b[44m" } else { "\x1b[34m" }),
        Color::Magenta => output.push_str(if background { "\x1b[45m" } else { "\x1b[35m" }),
        Color::Cyan => output.push_str(if background { "\x1b[46m" } else { "\x1b[36m" }),
        Color::Gray => output.push_str(if background { "\x1b[47m" } else { "\x1b[37m" }),
        Color::DarkGray => output.push_str(if background { "\x1b[100m" } else { "\x1b[90m" }),
        Color::LightRed => output.push_str(if background { "\x1b[101m" } else { "\x1b[91m" }),
        Color::LightGreen => output.push_str(if background { "\x1b[102m" } else { "\x1b[92m" }),
        Color::LightYellow => output.push_str(if background { "\x1b[103m" } else { "\x1b[93m" }),
        Color::LightBlue => output.push_str(if background { "\x1b[104m" } else { "\x1b[94m" }),
        Color::LightMagenta => output.push_str(if background { "\x1b[105m" } else { "\x1b[95m" }),
        Color::LightCyan => output.push_str(if background { "\x1b[106m" } else { "\x1b[96m" }),
        Color::White => output.push_str(if background { "\x1b[107m" } else { "\x1b[97m" }),
        Color::Indexed(value) => {
            let _ = write!(output, "\x1b[{};5;{}m", prefix, value);
        }
        Color::Rgb(red, green, blue) => {
            let _ = write!(output, "\x1b[{};2;{};{};{}m", prefix, red, green, blue);
        }
    }
}
