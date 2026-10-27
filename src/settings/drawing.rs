use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{
    input::CursorPosition,
    palette::TerminalPalette,
    prompt::{self, PromptSpan},
};

use super::SettingsState;

pub(super) const SETTING_ROW_START: u16 = 4;

pub(super) struct SettingsLayout {
    pub(super) inner: Rect,
    pub(super) value_column: u16,
    pub(super) palette: TerminalPalette,
}

pub(super) fn draw_prompt_value(
    buffer: &mut Buffer,
    column: u16,
    row: u16,
    value: &str,
    max_width: u16,
    selected: bool,
    palette: TerminalPalette,
) {
    let selection = palette.cursor_style(1.0).add_modifier(Modifier::BOLD);
    let mut offset: u16 = 0;
    for span in prompt::parse_literal(value) {
        let style = span_style(&span, selected, selection);
        offset += draw_text(
            buffer,
            column + offset,
            row,
            &span.text,
            max_width.saturating_sub(offset),
            style,
        );
    }
}

fn span_style(span: &PromptSpan, selected: bool, selection: Style) -> Style {
    if !selected {
        return span.style;
    }
    let mut style = span.style;
    style.bg = selection.bg;
    if style.fg.is_none() {
        style.fg = selection.fg;
    }
    style.add_modifier = style.add_modifier.union(selection.add_modifier);
    style
}

pub(super) fn draw_text(
    buffer: &mut Buffer,
    column: u16,
    row: u16,
    text: &str,
    max_width: u16,
    style: Style,
) -> u16 {
    let mut offset: u16 = 0;
    for grapheme in text.graphemes(true) {
        let width = UnicodeWidthStr::width(grapheme) as u16;
        if width == 0 || offset.saturating_add(width) > max_width {
            break;
        }
        buffer.set_string(column + offset, row, grapheme, style);
        offset += width;
    }
    offset
}

pub(super) fn fill_row(buffer: &mut Buffer, inner: Rect, row: u16, style: Style) {
    for column in inner.left()..inner.right() {
        if let Some(cell) = buffer.cell_mut((column, row)) {
            cell.set_symbol(" ").set_style(style);
        }
    }
}

pub(super) fn paint_editing_cursor(
    buffer: &mut Buffer,
    state: &SettingsState,
    layout: &SettingsLayout,
) -> CursorPosition {
    if !state.is_editing() {
        return CursorPosition { column: 0, row: 0 };
    }
    let row = layout
        .inner
        .top()
        .saturating_add(SETTING_ROW_START + state.selected_index() as u16);
    let value = state.value(state.selected_definition().key);
    let prefix = &value[..state.cursor()];
    let column = layout
        .value_column
        .saturating_add(prompt::width(&prompt::parse_literal(prefix)) as u16)
        .min(layout.inner.right().saturating_sub(1));
    if let Some(cell) = buffer.cell_mut((column, row)) {
        cell.set_symbol(" ")
            .set_style(layout.palette.cursor_style(1.0));
    }
    CursorPosition { column, row }
}
