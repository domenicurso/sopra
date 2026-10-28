use super::{
    drawing::{draw_text, fill},
    panel,
    state::SettingsState,
};
use crate::{input::TerminalSize, palette::TerminalPalette, scene::Frame};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Widget},
};

pub(super) fn build(state: &SettingsState, size: TerminalSize, palette: TerminalPalette) -> Frame {
    let area = Rect::new(0, 0, size.columns, size.rows);
    let dialog = dialog_area(size);
    let block = Block::default()
        .title(" Sopra Settings ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().add_modifier(Modifier::DIM));
    let inner = block.inner(dialog);
    let mut buffer = Buffer::empty(area);
    fill(&mut buffer, dialog, Style::default());
    block.render(dialog, &mut buffer);
    let sidebar_width = (inner.width / 4)
        .clamp(1, 16)
        .min(inner.width.saturating_sub(1));
    let sidebar = Rect::new(
        inner.left(),
        inner.top(),
        sidebar_width,
        inner.height.saturating_sub(3),
    );
    let content = Rect::new(
        inner.left().saturating_add(sidebar_width + 1),
        inner.top(),
        inner.width.saturating_sub(sidebar_width + 1),
        inner.height.saturating_sub(3),
    );
    let footer = Rect::new(
        inner.left(),
        inner.bottom().saturating_sub(2),
        inner.width,
        inner.height.min(2),
    );
    let dim = Style::default().add_modifier(Modifier::DIM);
    if inner.height >= 3 {
        for x in inner.left()..inner.right() {
            draw_text(&mut buffer, x, inner.bottom() - 3, "─", 1, dim);
        }
        for y in inner.top()..inner.bottom() - 3 {
            draw_text(&mut buffer, inner.left() + sidebar_width, y, "│", 1, dim);
        }
    }
    panel::draw_sidebar(&mut buffer, sidebar, state, palette);
    let cursor = panel::draw_settings(&mut buffer, content, state, palette);
    panel::draw_footer(&mut buffer, footer, state);
    Frame {
        buffer,
        clear_rows: Vec::new(),
        repaint_cells: Vec::new(),
        cursor,
        required_height: size.rows,
    }
}

fn dialog_area(size: TerminalSize) -> Rect {
    let width = size.columns.saturating_sub(2).clamp(1, 100);
    let height = size.rows.saturating_sub(2).clamp(1, 27);
    Rect::new(
        size.columns.saturating_sub(width) / 2,
        size.rows.saturating_sub(height) / 2,
        width,
        height,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        input::Key,
        settings::{config::Settings, state::SettingsState},
    };

    fn text(frame: &Frame) -> String {
        let mut result = String::new();
        for row in 0..frame.buffer.area.height {
            for column in 0..frame.buffer.area.width {
                result.push_str(frame.buffer[(column, row)].symbol());
            }
            result.push('\n');
        }
        result
    }

    #[test]
    fn sidebar_controls_and_selection_remain_visible() {
        let mut state = SettingsState::test(Settings::default());
        let size = TerminalSize::new(90, 24);
        let first = text(&build(&state, size, TerminalPalette::default()));
        assert!(first.contains("Prompts") && first.contains("Prompt shown while editing"));
        assert!(first.contains("│") && first.contains("─"));
        state.handle_key(Key::Right);
        let editor = text(&build(&state, size, TerminalPalette::default()));
        assert!(editor.contains("Cursor animation") && editor.contains("[ On ]"));
        state.handle_key(Key::Tab);
        let editor = text(&build(&state, size, TerminalPalette::default()));
        assert!(editor.contains("Syntax highlighting"));
    }
}
