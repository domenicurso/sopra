use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
};

use super::{
    controls::{SettingDisplay, draw_setting},
    drawing::{draw_text, fill},
    schema::TABS,
    state::SettingsState,
};
use crate::{input::CursorPosition, palette::TerminalPalette};

pub(super) fn draw_sidebar(
    buffer: &mut Buffer,
    area: Rect,
    state: &SettingsState,
    palette: TerminalPalette,
) {
    for (index, tab) in TABS.iter().enumerate() {
        let row = area.top().saturating_add(1 + index as u16 * 2);
        if row >= area.bottom() {
            break;
        }
        let style = if index == state.tab_index() {
            palette.cursor_style(1.0)
        } else {
            Style::default()
        };
        fill(buffer, Rect::new(area.left(), row, area.width, 1), style);
        draw_text(
            buffer,
            area.left().saturating_add(1),
            row,
            tab.label,
            area.width.saturating_sub(2),
            style,
        );
    }
}

pub(super) fn draw_settings(
    buffer: &mut Buffer,
    area: Rect,
    state: &SettingsState,
    palette: TerminalPalette,
) -> CursorPosition {
    let visible = (area.height / 5).max(1) as usize;
    let first = state
        .selected_index()
        .saturating_add(1)
        .saturating_sub(visible);
    let mut cursor = CursorPosition { column: 0, row: 0 };
    for (index, definition) in state
        .current_tab()
        .settings
        .iter()
        .enumerate()
        .skip(first)
        .take(visible)
    {
        let top = area.top().saturating_add((index - first) as u16 * 5);
        if top.saturating_add(3) >= area.bottom() {
            break;
        }
        let row = Rect::new(area.left(), top, area.width, 5);
        let selected = index == state.selected_index();
        if let Some(position) = draw_setting(
            buffer,
            row,
            state,
            SettingDisplay {
                definition: *definition,
                selected,
                palette,
            },
        ) {
            cursor = position;
        }
    }
    cursor
}

pub(super) fn draw_footer(buffer: &mut Buffer, area: Rect, state: &SettingsState) {
    let style = Style::default().add_modifier(Modifier::DIM);
    if area.height == 0 {
        return;
    }
    let message = state
        .error()
        .map_or_else(|| state.config_label(), ToString::to_string);
    draw_text(
        buffer,
        area.left().saturating_add(1),
        area.top(),
        &message,
        area.width.saturating_sub(2),
        style,
    );
    if area.height < 2 {
        return;
    }
    let help = if state.is_editing() {
        "Enter save   Esc cancel"
    } else {
        "←/→ section   Tab/Shift-Tab select   Enter edit/toggle   Esc close"
    };
    draw_text(
        buffer,
        area.left().saturating_add(1),
        area.top() + 1,
        help,
        area.width.saturating_sub(2),
        style,
    );
}
