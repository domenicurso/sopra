use std::fmt::Write as _;

use ratatui::{
    buffer::Cell,
    style::{Color, Modifier, Style},
};
use unicode_width::UnicodeWidthStr;

use crate::model::RenderedFrame;

pub(crate) fn write_row(output: &mut String, frame: &RenderedFrame, row: u16) {
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
