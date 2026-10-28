use super::{drawing::draw_text, schema::SettingKind, state::SettingsState};
use crate::{input::CursorPosition, palette::TerminalPalette};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub(super) struct SettingDisplay {
    pub(super) definition: super::schema::SettingDefinition,
    pub(super) selected: bool,
    pub(super) palette: TerminalPalette,
}

pub(super) fn draw_setting(
    buffer: &mut Buffer,
    area: Rect,
    state: &SettingsState,
    display: SettingDisplay,
) -> Option<CursorPosition> {
    let x = area.left().saturating_add(1);
    let width = area.width.saturating_sub(2);
    let definition = display.definition;
    draw_text(
        buffer,
        x,
        area.top() + 1,
        definition.label,
        width,
        Style::default().fg(Color::White),
    );
    draw_text(
        buffer,
        x,
        area.top() + 2,
        definition.description,
        width,
        Style::default().add_modifier(Modifier::DIM),
    );
    draw_input(buffer, area, state, display)
}

fn draw_input(
    buffer: &mut Buffer,
    area: Rect,
    state: &SettingsState,
    display: SettingDisplay,
) -> Option<CursorPosition> {
    let x = area.left().saturating_add(1);
    let width = area.width.saturating_sub(2);
    let definition = display.definition;
    let style = if display.selected {
        display.palette.cursor_style(1.0)
    } else {
        Style::default().add_modifier(Modifier::DIM)
    };
    let original = state.value(definition.key);
    let value = original.replace('\n', "\\n");
    let editing = display.selected && state.is_editing();
    let prefix = if editing {
        original[..state.cursor()].replace('\n', "\\n")
    } else {
        String::new()
    };
    let (visible, cursor_offset) = scroll_value(&value, &prefix, width.saturating_sub(4));
    let shown = if definition.kind == SettingKind::Toggle {
        format!("[ {value} ]")
    } else {
        format!("[ {visible} ]")
    };
    draw_text(buffer, x, area.top() + 3, &shown, width, style);
    if !editing {
        return None;
    }
    let column = x
        .saturating_add(2)
        .saturating_add(cursor_offset)
        .min(area.right().saturating_sub(2));
    let row = area.top() + 3;
    if let Some(cell) = buffer.cell_mut((column, row)) {
        cell.set_style(Style::default());
    }
    Some(CursorPosition { column, row })
}

fn scroll_value<'a>(value: &'a str, prefix: &str, width: u16) -> (&'a str, u16) {
    let mut prefix_width = UnicodeWidthStr::width(prefix);
    let mut start = 0;
    for grapheme in prefix.graphemes(true) {
        if prefix_width < width as usize {
            break;
        }
        prefix_width = prefix_width.saturating_sub(UnicodeWidthStr::width(grapheme));
        start += grapheme.len();
    }
    (&value[start..], prefix_width as u16)
}
