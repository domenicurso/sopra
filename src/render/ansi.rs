use ratatui::{
    buffer::{Buffer, Cell},
    style::{Color, Modifier, Style},
};

#[derive(Clone, Copy)]
enum TerminalColor {
    Palette(u8),
    Indexed(u8),
    Rgb(u8, u8, u8),
}

pub(crate) fn region_highlight_style(style: Style) -> Option<String> {
    let mut parts = vec![format!("fg={}", style_color(style.fg?)?)];
    for (modifier, name) in [
        (Modifier::BOLD, "bold"),
        (Modifier::DIM, "dim"),
        (Modifier::UNDERLINED, "underline"),
    ] {
        if style.add_modifier.contains(modifier) {
            parts.push(name.to_string());
        }
    }
    Some(parts.join(","))
}

fn style_color(color: Color) -> Option<String> {
    Some(match terminal_color(color)? {
        TerminalColor::Palette(index) | TerminalColor::Indexed(index) => index.to_string(),
        TerminalColor::Rgb(red, green, blue) => format!("#{red:02x}{green:02x}{blue:02x}"),
    })
}

pub(super) fn cell_at(buffer: &Buffer, column: u16, row: u16) -> Cell {
    buffer.cell((column, row)).cloned().unwrap_or(Cell::EMPTY)
}

pub(super) fn move_to(output: &mut Vec<u8>, column: u16, row: u16) {
    output.extend_from_slice(b"\x1b[u");
    if row > 0 {
        output.extend_from_slice(format!("\x1b[{}B", row).as_bytes());
    }
    output.extend_from_slice(format!("\x1b[{}G", column + 1).as_bytes());
}

pub(super) fn paint_cell(output: &mut Vec<u8>, column: u16, row: u16, cell: &Cell) {
    move_to(output, column, row);
    output.extend_from_slice(b"\x1b[0m");
    paint_style(output, cell);
    output.extend_from_slice(if cell.symbol().is_empty() {
        b" "
    } else {
        cell.symbol().as_bytes()
    });
}

fn paint_style(output: &mut Vec<u8>, cell: &Cell) {
    let mut params = Vec::new();
    let modifier = cell.modifier;
    for (flag, code) in [
        (Modifier::BOLD, "1"),
        (Modifier::DIM, "2"),
        (Modifier::ITALIC, "3"),
        (Modifier::UNDERLINED, "4"),
        (Modifier::SLOW_BLINK, "5"),
        (Modifier::RAPID_BLINK, "6"),
        (Modifier::REVERSED, "7"),
        (Modifier::CROSSED_OUT, "9"),
    ] {
        if modifier.contains(flag) {
            params.push(code);
        }
    }
    if !params.is_empty() {
        output.extend_from_slice(format!("\x1b[{}m", params.join(";")).as_bytes());
    }
    paint_color(output, cell.fg, true);
    paint_color(output, cell.bg, false);
}

fn paint_color(output: &mut Vec<u8>, color: Color, foreground: bool) {
    let prefix = if foreground { 38 } else { 48 };
    match terminal_color(color) {
        None => {}
        Some(TerminalColor::Palette(index)) => paint_basic_color(output, prefix, index),
        Some(TerminalColor::Indexed(index)) => {
            output.extend_from_slice(format!("\x1b[{};5;{}m", prefix, index).as_bytes())
        }
        Some(TerminalColor::Rgb(red, green, blue)) => output
            .extend_from_slice(format!("\x1b[{};2;{};{};{}m", prefix, red, green, blue).as_bytes()),
    }
}

fn terminal_color(color: Color) -> Option<TerminalColor> {
    Some(match color {
        Color::Reset => return None,
        Color::Black => TerminalColor::Palette(0),
        Color::Red => TerminalColor::Palette(1),
        Color::Green => TerminalColor::Palette(2),
        Color::Yellow => TerminalColor::Palette(3),
        Color::Blue => TerminalColor::Palette(4),
        Color::Magenta => TerminalColor::Palette(5),
        Color::Cyan => TerminalColor::Palette(6),
        Color::Gray => TerminalColor::Palette(7),
        Color::DarkGray => TerminalColor::Palette(8),
        Color::LightRed => TerminalColor::Palette(9),
        Color::LightGreen => TerminalColor::Palette(10),
        Color::LightYellow => TerminalColor::Palette(11),
        Color::LightBlue => TerminalColor::Palette(12),
        Color::LightMagenta => TerminalColor::Palette(13),
        Color::LightCyan => TerminalColor::Palette(14),
        Color::White => TerminalColor::Palette(15),
        Color::Indexed(index) => TerminalColor::Indexed(index),
        Color::Rgb(red, green, blue) => TerminalColor::Rgb(red, green, blue),
    })
}

fn paint_basic_color(output: &mut Vec<u8>, prefix: u8, color: u8) {
    let (base, offset) = if color < 8 {
        (30, color)
    } else {
        (90, color - 8)
    };
    let code = if prefix == 38 {
        base + offset
    } else {
        base + 10 + offset
    };
    output.extend_from_slice(format!("\x1b[{}m", code).as_bytes());
}
