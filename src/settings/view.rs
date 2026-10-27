use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Widget},
};

use crate::{input::TerminalSize, palette::TerminalPalette, scene::Frame};

use super::{
    drawing::{SETTING_ROW_START, SettingsLayout, paint_editing_cursor},
    panel,
    state::SettingsState,
};

const DIALOG_WIDTH: u16 = 82;
const MAX_DIALOG_HEIGHT: u16 = 18;
const VALUE_COLUMN_OFFSET: u16 = 26;

pub(super) fn build(state: &SettingsState, size: TerminalSize, palette: TerminalPalette) -> Frame {
    let area = Rect::new(0, 0, size.columns, size.rows);
    let dialog = dialog_area(state.current_tab().settings.len(), size);
    let block = Block::default()
        .title(" Sopra Settings ")
        .title_style(palette.cursor_style(1.0).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().add_modifier(Modifier::DIM));
    let inner = block.inner(dialog);
    let mut buffer = Buffer::empty(area);
    panel::fill_dialog(&mut buffer, dialog);
    block.render(dialog, &mut buffer);
    let selection = palette.cursor_style(1.0).add_modifier(Modifier::BOLD);
    panel::draw_tabs(&mut buffer, state.tab_index(), inner, selection);
    panel::draw_tab_description(&mut buffer, state.current_tab(), inner);
    panel::draw_rule(&mut buffer, inner, 3);
    let layout = SettingsLayout {
        inner,
        value_column: inner.left().saturating_add(VALUE_COLUMN_OFFSET),
        palette,
    };
    panel::draw_settings(&mut buffer, state, &layout, selection);
    panel::draw_rule(
        &mut buffer,
        inner,
        SETTING_ROW_START + state.current_tab().settings.len() as u16,
    );
    panel::draw_selected_description(&mut buffer, state, inner);
    panel::draw_footer(&mut buffer, state, inner);
    let cursor = paint_editing_cursor(&mut buffer, state, &layout);
    Frame {
        buffer,
        clear_rows: Vec::new(),
        repaint_cells: Vec::new(),
        cursor,
        required_height: size.rows,
    }
}

fn dialog_area(setting_count: usize, size: TerminalSize) -> Rect {
    let width = size.columns.saturating_sub(2).clamp(1, DIALOG_WIDTH);
    let desired_height = 2 + super::drawing::SETTING_ROW_START + setting_count as u16 + 5;
    let available_height = size.rows.saturating_sub(2).max(1);
    let height = desired_height
        .min(MAX_DIALOG_HEIGHT)
        .clamp(1, available_height);
    Rect::new(
        size.columns.saturating_sub(width) / 2,
        size.rows.saturating_sub(height) / 2,
        width,
        height,
    )
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use super::*;
    use crate::settings::{config::Settings, state::SettingsState};

    #[test]
    fn settings_dialog_is_centered_and_keeps_prompt_ansi() {
        let state = SettingsState::test(Settings {
            prompt: "\x1b[32mgreen\x1b[0m".to_string(),
            ..Settings::default()
        });
        let frame = build(
            &state,
            TerminalSize::new(100, 40),
            TerminalPalette::default(),
        );
        assert_eq!(frame.buffer[(9, 12)].symbol(), "╭");
        assert_eq!(frame.buffer[(36, 17)].symbol(), "g");
        assert_eq!(frame.buffer[(36, 17)].fg, Color::Green);
        assert_eq!(frame.buffer[(10, 13)].symbol(), "P");
        assert_eq!(frame.buffer[(10, 16)].symbol(), "─");
    }

    #[test]
    fn settings_show_the_literal_format_string() {
        let state = SettingsState::test(Settings {
            prompt: "[dim]{path}[/]".to_string(),
            ..Settings::default()
        });
        let frame = build(
            &state,
            TerminalSize::new(100, 40),
            TerminalPalette::default(),
        );
        let text = (36..65)
            .map(|column| frame.buffer[(column, 17)].symbol())
            .collect::<String>();
        assert!(text.contains("[dim]{path}[/]"));
    }

    #[test]
    fn selected_setting_description_is_rendered() {
        let state = SettingsState::test(Settings::default());
        let frame = build(
            &state,
            TerminalSize::new(100, 40),
            TerminalPalette::default(),
        );
        let text = (10..90)
            .map(|column| frame.buffer[(column, 23)].symbol())
            .collect::<String>();
        assert!(text.contains("Prompt shown while editing"));
    }
}
