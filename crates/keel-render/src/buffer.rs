use keel_core::{CanvasBuffer, CanvasCell, CellStyle, TerminalPoint};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteCursor {
    pub row: u16,
    pub column: u16,
}

impl WriteCursor {
    pub fn at(point: TerminalPoint) -> Self {
        Self {
            row: point.row,
            column: point.column,
        }
    }

    pub fn point(self) -> TerminalPoint {
        TerminalPoint {
            row: self.row,
            column: self.column,
        }
    }
}

pub fn write_text(
    buffer: &mut CanvasBuffer,
    cursor: &mut WriteCursor,
    text: &str,
    style: CellStyle,
) -> TerminalPoint {
    for grapheme in UnicodeSegmentation::graphemes(text, true) {
        write_grapheme(buffer, cursor, grapheme, style);
    }

    cursor.point()
}

pub fn write_grapheme(
    buffer: &mut CanvasBuffer,
    cursor: &mut WriteCursor,
    grapheme: &str,
    style: CellStyle,
) {
    if buffer.size.columns == 0 || buffer.size.rows == 0 {
        return;
    }

    if grapheme == "\n" {
        cursor.row = cursor.row.saturating_add(1).min(buffer.size.rows.saturating_sub(1));
        cursor.column = 0;
        return;
    }

    let width = grapheme_width(grapheme) as u16;
    if cursor.column > 0 && cursor.column.saturating_add(width) > buffer.size.columns {
        cursor.row = cursor.row.saturating_add(1).min(buffer.size.rows.saturating_sub(1));
        cursor.column = 0;
    }

    if cursor.row >= buffer.size.rows || cursor.column >= buffer.size.columns {
        return;
    }

    let row = &mut buffer.rows[cursor.row as usize];
    row.cells[cursor.column as usize] = CanvasCell {
        symbol: grapheme.to_string(),
        style,
    };

    for offset in 1..width {
        let column = cursor.column.saturating_add(offset);
        if column >= buffer.size.columns {
            break;
        }
        row.cells[column as usize] = CanvasCell {
            symbol: " ".to_string(),
            style,
        };
    }

    cursor.column = cursor.column.saturating_add(width);
    if cursor.column >= buffer.size.columns {
        cursor.row = cursor.row.saturating_add(1).min(buffer.size.rows.saturating_sub(1));
        cursor.column = 0;
    }
}

pub fn grapheme_width(grapheme: &str) -> usize {
    if grapheme.chars().count() == 1 {
        return UnicodeWidthChar::width(grapheme.chars().next().unwrap())
            .unwrap_or(0)
            .max(1);
    }

    UnicodeWidthStr::width(grapheme).max(1)
}
