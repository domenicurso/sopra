use ratatui::{
    buffer::{Buffer, Cell},
    style::{Color, Modifier},
};

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
    match color {
        Color::Reset => {}
        Color::Black => paint_basic_color(output, prefix, 0),
        Color::Red => paint_basic_color(output, prefix, 1),
        Color::Green => paint_basic_color(output, prefix, 2),
        Color::Yellow => paint_basic_color(output, prefix, 3),
        Color::Blue => paint_basic_color(output, prefix, 4),
        Color::Magenta => paint_basic_color(output, prefix, 5),
        Color::Cyan => paint_basic_color(output, prefix, 6),
        Color::Gray => paint_basic_color(output, prefix, 7),
        Color::DarkGray => paint_basic_color(output, prefix, 8),
        Color::LightRed => paint_basic_color(output, prefix, 9),
        Color::LightGreen => paint_basic_color(output, prefix, 10),
        Color::LightYellow => paint_basic_color(output, prefix, 11),
        Color::LightBlue => paint_basic_color(output, prefix, 12),
        Color::LightMagenta => paint_basic_color(output, prefix, 13),
        Color::LightCyan => paint_basic_color(output, prefix, 14),
        Color::White => paint_basic_color(output, prefix, 15),
        Color::Indexed(index) => {
            output.extend_from_slice(format!("\x1b[{};5;{}m", prefix, index).as_bytes())
        }
        Color::Rgb(red, green, blue) => output
            .extend_from_slice(format!("\x1b[{};2;{};{};{}m", prefix, red, green, blue).as_bytes()),
    }
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
