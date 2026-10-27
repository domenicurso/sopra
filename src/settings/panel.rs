use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
};

use crate::palette::TerminalPalette;

use super::{
    drawing::{SETTING_ROW_START, SettingsLayout, draw_prompt_value, draw_text, fill_row},
    schema::{SettingKind, TABS, TabDefinition},
    state::SettingsState,
};

pub(super) fn fill_dialog(buffer: &mut Buffer, dialog: Rect) {
    let style = Style::default().add_modifier(Modifier::DIM);
    for row in dialog.top()..dialog.bottom() {
        for column in dialog.left()..dialog.right() {
            if let Some(cell) = buffer.cell_mut((column, row)) {
                cell.set_symbol(" ").set_style(style);
            }
        }
    }
}
pub(super) fn draw_tabs(buffer: &mut Buffer, selected: usize, inner: Rect, selection: Style) {
    let mut column = inner.left();
    for (index, tab) in TABS.iter().enumerate() {
        let style = if index == selected {
            selection
        } else {
            Style::default().add_modifier(Modifier::DIM)
        };
        column = column.saturating_add(draw_text(
            buffer,
            column,
            inner.top(),
            tab.label,
            inner.right().saturating_sub(column),
            style,
        ));
        if index + 1 < TABS.len() {
            column = column.saturating_add(draw_text(
                buffer,
                column,
                inner.top(),
                "   ",
                inner.right().saturating_sub(column),
                Style::default().add_modifier(Modifier::DIM),
            ));
        }
    }
}
pub(super) fn draw_tab_description(buffer: &mut Buffer, tab: &TabDefinition, inner: Rect) {
    draw_text(
        buffer,
        inner.left(),
        inner.top().saturating_add(2),
        tab.description,
        inner.width,
        Style::default().add_modifier(Modifier::DIM),
    );
}

pub(super) fn draw_rule(buffer: &mut Buffer, inner: Rect, offset: u16) {
    let row = inner.top().saturating_add(offset);
    let style = Style::default().add_modifier(Modifier::DIM);
    for column in inner.left()..inner.right() {
        buffer.set_string(column, row, "─", style);
    }
}

pub(super) fn draw_settings(
    buffer: &mut Buffer,
    state: &SettingsState,
    layout: &SettingsLayout,
    selection: Style,
) {
    for (index, definition) in state.current_tab().settings.iter().enumerate() {
        let row = layout
            .inner
            .top()
            .saturating_add(SETTING_ROW_START + index as u16);
        if row >= layout.inner.bottom() {
            continue;
        }
        let selected = state.selected_index() == index;
        if selected {
            fill_row(buffer, layout.inner, row, selection);
        }
        let label_style = if selected {
            selection
        } else {
            Style::default()
        };
        draw_text(
            buffer,
            layout.inner.left(),
            row,
            definition.label,
            24,
            label_style,
        );
        draw_value(
            buffer,
            layout.value_column,
            row,
            ValueDisplay {
                text: state.value(definition.key),
                max_width: layout.inner.right().saturating_sub(layout.value_column),
                selected,
                kind: definition.kind,
                palette: layout.palette,
            },
        );
    }
}

pub(super) fn draw_selected_description(buffer: &mut Buffer, state: &SettingsState, inner: Rect) {
    let row = inner
        .top()
        .saturating_add(SETTING_ROW_START + state.current_tab().settings.len() as u16 + 2);
    let description = format!("↳ {}", state.selected_definition().description);
    draw_text(
        buffer,
        inner.left(),
        row,
        &description,
        inner.width,
        Style::default().add_modifier(Modifier::DIM),
    );
}

fn draw_value(buffer: &mut Buffer, column: u16, row: u16, display: ValueDisplay) {
    if display.kind == SettingKind::Text {
        draw_prompt_value(
            buffer,
            column,
            row,
            &display.text,
            display.max_width,
            display.selected,
            display.palette,
        );
        return;
    }
    let style = if display.selected {
        display
            .palette
            .cursor_style(1.0)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    draw_text(buffer, column, row, &display.text, display.max_width, style);
}

struct ValueDisplay {
    text: String,
    max_width: u16,
    selected: bool,
    kind: SettingKind,
    palette: TerminalPalette,
}

pub(super) fn draw_footer(buffer: &mut Buffer, state: &SettingsState, inner: Rect) {
    if inner.height < 3 {
        return;
    }
    let status_row = inner.bottom().saturating_sub(2);
    let footer_row = inner.bottom().saturating_sub(1);
    let status_style = if state.error().is_some() {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::DIM)
    };
    let status = state
        .error()
        .map_or_else(|| state.config_label(), ToString::to_string);
    draw_text(
        buffer,
        inner.left(),
        status_row,
        &status,
        inner.width,
        status_style,
    );
    let footer = if state.is_editing() {
        "Enter save   Esc cancel"
    } else {
        "←→ tabs   ↑↓ select   Enter edit   Esc close"
    };
    draw_text(
        buffer,
        inner.left(),
        footer_row,
        footer,
        inner.width,
        Style::default().add_modifier(Modifier::DIM),
    );
}
