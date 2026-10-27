use ratatui::layout::Rect;

use crate::{editor::EditorState, input::TerminalSize};

use super::overlay_layout;

pub(super) struct EditorLayout {
    pub(super) area: Rect,
    pub(super) row: u16,
    pub(super) prompt_row: u16,
    pub(super) column: u16,
    pub(super) line_column: u16,
    pub(super) cursor_column: u16,
    pub(super) rprompt_column: u16,
    pub(super) overlay_row: u16,
    pub(super) overlay_height: u16,
    pub(super) overlay_width: u16,
    pub(super) overlay_above: bool,
    pub(super) completion_token_width: u16,
    pub(super) footer_hint: String,
    pub(super) footer: String,
}

impl EditorLayout {
    pub(super) fn new(
        editor: &EditorState,
        size: TerminalSize,
        prompt_width: usize,
        rprompt_width: usize,
    ) -> Self {
        let area = Rect::new(0, 0, size.columns, size.rows);
        let prompt_row = editor.anchor().row.min(size.rows.saturating_sub(1));
        let prompt_breaks = editor.prompt().matches('\n').count() as u16;
        let row = prompt_row
            .saturating_add(prompt_breaks)
            .min(size.rows.saturating_sub(1));
        let column = editor.anchor().column.min(size.columns.saturating_sub(1));
        let line_column = if row == prompt_row { column } else { 0 }
            .saturating_add(prompt_width as u16)
            .min(size.columns.saturating_sub(1));
        let cursor_column = line_column
            .saturating_add(editor.cursor_display_width())
            .min(size.columns.saturating_sub(1));
        let buffer_end = line_column.saturating_add(editor.buffer_display_width());
        let cursor_end = cursor_column.saturating_add(editor.cursor_cell_width());
        let occupied = if row == prompt_row {
            buffer_end.max(cursor_end)
        } else {
            crate::prompt::lines::widths(&crate::prompt::parse(editor.prompt()))[0] as u16
        };
        let rprompt_column = rprompt_column(size.columns, occupied, rprompt_width);
        let completion_token_width = editor.completion_token_width();
        let placement = overlay_layout::for_editor(editor, size, row);
        let overlay_row = placement.as_ref().map_or(0, |value| value.row);
        let overlay_height = placement.as_ref().map_or(0, |value| value.height);
        Self {
            area,
            row,
            prompt_row,
            column,
            line_column,
            cursor_column,
            rprompt_column,
            overlay_row,
            overlay_height,
            overlay_width: 0,
            overlay_above: placement.is_some_and(|value| value.above),
            completion_token_width,
            footer_hint: String::new(),
            footer: String::new(),
        }
    }

    pub(super) fn overlay_column(&self) -> u16 {
        self.cursor_column
            .saturating_sub(self.completion_token_width)
            .saturating_sub(2)
            .min(self.area.width.saturating_sub(self.overlay_width))
    }

    pub(super) fn connector_column(&self) -> u16 {
        if self.overlay_width >= 3 {
            self.cursor_column
                .saturating_sub(self.overlay_column())
                .clamp(1, self.overlay_width.saturating_sub(2))
        } else {
            0
        }
    }

    pub(super) fn required_height(&self) -> u16 {
        let line_height = self.row.saturating_add(1);
        let overlay_height = self.overlay_row.saturating_add(self.overlay_height);
        line_height.max(overlay_height)
    }
}

fn rprompt_column(columns: u16, occupied_end: u16, width: usize) -> u16 {
    let width = width.min(columns as usize) as u16;
    if width == 0 || width >= columns {
        return columns;
    }
    let column = columns.saturating_sub(width);
    if occupied_end <= column {
        column
    } else {
        columns
    }
}

#[cfg(test)]
mod tests {
    use super::EditorLayout;
    use crate::{
        editor::{EditorConfig, EditorState},
        input::{CursorPosition, TerminalSize},
        palette::TerminalPalette,
    };

    fn editor(cursor_chars: usize) -> EditorState {
        editor_with_buffer(33, cursor_chars)
    }

    fn editor_with_buffer(buffer_chars: usize, cursor_chars: usize) -> EditorState {
        let buffer = "1".repeat(buffer_chars);
        EditorState::new(EditorConfig {
            buffer,
            cursor_chars,
            arm_completion: false,
            suppress_completion: true,
            completion_enabled: false,
            syntax_highlighting: false,
            cursor_animation: false,
            prompt: "$ ".to_string(),
            rprompt: "RIGHT".to_string(),
            anchor: CursorPosition { row: 0, column: 0 },
            size: TerminalSize::new(40, 24),
            cwd: std::path::PathBuf::new(),
            palette: TerminalPalette::default(),
        })
    }

    #[test]
    fn multiline_input_starts_at_the_left_edge_after_a_nonzero_anchor() {
        let editor = EditorState::new(EditorConfig {
            buffer: "abc".into(),
            cursor_chars: 3,
            arm_completion: false,
            suppress_completion: true,
            completion_enabled: false,
            syntax_highlighting: false,
            cursor_animation: false,
            prompt: "first\n$ ".into(),
            rprompt: "right".into(),
            anchor: CursorPosition { row: 0, column: 7 },
            size: TerminalSize::new(40, 24),
            cwd: std::path::PathBuf::new(),
            palette: TerminalPalette::default(),
        });
        let layout = EditorLayout::new(&editor, TerminalSize::new(40, 24), 2, 5);
        assert_eq!(
            (
                layout.prompt_row,
                layout.row,
                layout.line_column,
                layout.cursor_column
            ),
            (0, 1, 2, 5)
        );
    }

    #[test]
    fn right_prompt_does_not_cover_the_end_cursor_at_an_exact_fit() {
        let size = TerminalSize::new(40, 24);
        let layout = EditorLayout::new(&editor(33), size, 2, 5);

        assert_eq!(layout.rprompt_column, size.columns);
    }

    #[test]
    fn right_prompt_ends_at_the_terminal_edge_when_there_is_room() {
        let layout =
            EditorLayout::new(&editor_with_buffer(29, 29), TerminalSize::new(40, 24), 2, 5);

        assert_eq!(layout.rprompt_column, 35);
    }

    #[test]
    fn right_prompt_is_hidden_when_the_terminal_has_no_safe_right_edge() {
        let layout = EditorLayout::new(&editor(0), TerminalSize::new(1, 24), 2, 5);

        assert_eq!(layout.rprompt_column, 1);
    }
}
