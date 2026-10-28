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
    pub(super) cursor_row: u16,
    pub(super) end_row: u16,
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
    pub(super) fn new(editor: &EditorState, size: TerminalSize, rprompt_width: usize) -> Self {
        let area = Rect::new(0, 0, size.columns, size.rows);
        let prompt_row = editor.anchor().row.min(size.rows.saturating_sub(1));
        let column = editor.anchor().column.min(size.columns.saturating_sub(1));
        let (start, cursor, end) = command_positions(editor, size);
        let row = start.row;
        let line_column = start.column;
        let occupied = if row == prompt_row {
            if end.row > prompt_row {
                size.columns
            } else {
                end.column
                    .max(cursor.column.saturating_add(editor.cursor_cell_width()))
            }
        } else {
            crate::prompt::lines::widths(&crate::prompt::parse(editor.prompt()))[0]
                .min(size.columns as usize) as u16
        };
        let rprompt_column = rprompt_column(size.columns, occupied, rprompt_width);
        let completion_token_width = editor.completion_token_width();
        let placement = overlay_layout::for_editor(editor, size, cursor.row);
        let overlay_row = placement.as_ref().map_or(0, |value| value.row);
        let overlay_height = placement.as_ref().map_or(0, |value| value.height);
        Self {
            area,
            row,
            prompt_row,
            column,
            line_column,
            cursor_column: cursor.column,
            cursor_row: cursor.row,
            end_row: end.row,
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
        let line_height = self.end_row.saturating_add(1);
        let overlay_height = self.overlay_row.saturating_add(self.overlay_height);
        line_height.max(overlay_height)
    }
}

fn command_positions(
    editor: &EditorState,
    size: TerminalSize,
) -> (
    crate::input::CursorPosition,
    crate::input::CursorPosition,
    crate::input::CursorPosition,
) {
    let mut flow = super::text_flow::TextFlow::new(
        crate::input::CursorPosition {
            row: editor.anchor().row,
            column: editor.anchor().column,
        },
        size.columns,
    );
    let prompt = crate::prompt::parse(editor.prompt());
    flow.prompt(&prompt);
    let start = flow.cursor();
    let mut cursor_flow = flow;
    let secondary = crate::prompt::parse(editor.secondary_prompt());
    cursor_flow.command(&editor.buffer()[..editor.cursor_byte()], &secondary);
    let cursor = cursor_flow.cursor();
    flow.command(editor.buffer(), &secondary);
    let end = flow.cursor();
    (start, cursor, end)
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
#[path = "editor_layout_tests.rs"]
mod tests;
