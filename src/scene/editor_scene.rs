use std::time::Instant;

use ratatui::layout::Rect;

use crate::{completion::CompletionItem, editor::EditorState, input::TerminalSize};

use super::overlay::{OverlayElement, overlay_width};
use super::{
    Element, MAX_OVERLAY_ITEMS, Scene,
    elements::{CommandElement, CursorElement, PromptElement},
};

pub(super) fn build(editor: &EditorState, size: TerminalSize, now: Instant) -> Scene {
    let items = editor.visible_items().to_vec();
    let prompt = crate::prompt::parse(editor.prompt());
    let layout = EditorLayout::new(editor, size, &items, crate::prompt::width(&prompt));
    let elements = editor_elements(editor, &layout, now, items, prompt);
    let cursor_cells = (0..editor.cursor_cell_width())
        .map(|offset| (layout.cursor_column.saturating_add(offset), layout.row))
        .filter(|(column, _)| *column < size.columns)
        .collect();
    Scene::new(layout.area, clear_rows(&layout), cursor_cells, elements)
}

struct EditorLayout {
    area: Rect,
    row: u16,
    column: u16,
    line_column: u16,
    cursor_column: u16,
    overlay_visible: bool,
    overlay_row: u16,
    overlay_height: u16,
    overlay_width: u16,
    overlay_above: bool,
    completion_token_width: u16,
}

impl EditorLayout {
    fn new(
        editor: &EditorState,
        size: TerminalSize,
        items: &[CompletionItem],
        prompt_width: usize,
    ) -> Self {
        let area = Rect::new(0, 0, size.columns, size.rows);
        let row = editor.anchor().row.min(size.rows.saturating_sub(1));
        let column = editor.anchor().column.min(size.columns.saturating_sub(1));
        let line_column = column
            .saturating_add(prompt_width as u16)
            .min(size.columns.saturating_sub(1));
        let cursor_column = line_column
            .saturating_add(editor.cursor_display_width())
            .min(size.columns.saturating_sub(1));
        let completion_token_width = editor.completion_token_width();
        let overlay_visible = editor.overlay_visible() && !items.is_empty();
        let overlay_height = if overlay_visible {
            (items.len().clamp(1, MAX_OVERLAY_ITEMS) as u16).saturating_add(2)
        } else {
            0
        };
        let overlay_width = overlay_width(items, size.columns);
        let below_row = row.saturating_add(2);
        let below_fits = !overlay_visible || below_row.saturating_add(overlay_height) <= size.rows;
        let overlay_row = if below_fits {
            below_row
        } else {
            row.saturating_sub(overlay_height.saturating_add(1))
        };
        Self {
            area,
            row,
            column,
            line_column,
            cursor_column,
            overlay_visible,
            overlay_row,
            overlay_height,
            overlay_width,
            overlay_above: overlay_visible && !below_fits,
            completion_token_width,
        }
    }
}

fn editor_elements(
    editor: &EditorState,
    layout: &EditorLayout,
    now: Instant,
    items: Vec<CompletionItem>,
    prompt: Vec<crate::prompt::PromptSpan>,
) -> Vec<Box<dyn Element>> {
    let mut elements: Vec<Box<dyn Element>> = vec![
        Box::new(PromptElement {
            column: layout.column,
            row: layout.row,
            spans: prompt,
        }),
        Box::new(CommandElement {
            column: layout.line_column,
            row: layout.row,
            buffer: editor.buffer().to_string(),
            spans: editor.syntax().to_vec(),
        }),
    ];
    if layout.overlay_visible {
        elements.push(Box::new(OverlayElement {
            area: Rect::new(
                layout.overlay_column(),
                layout.overlay_row,
                layout.overlay_width,
                layout.overlay_height,
            ),
            items,
            selected: editor.selected_index(),
            connector_column: layout.connector_column(),
            connector: if layout.overlay_above { "┬" } else { "┴" },
        }));
    }
    elements.push(Box::new(CursorElement {
        column: layout.cursor_column,
        row: layout.row,
        width: editor.cursor_cell_width(),
        style: editor.cursor_style(now),
    }));
    elements
}

impl EditorLayout {
    fn overlay_column(&self) -> u16 {
        self.cursor_column
            .saturating_sub(self.completion_token_width)
            .saturating_sub(2)
            .min(self.area.width.saturating_sub(self.overlay_width))
    }

    fn connector_column(&self) -> u16 {
        if self.overlay_width >= 3 {
            self.cursor_column
                .saturating_sub(self.overlay_column())
                .clamp(1, self.overlay_width.saturating_sub(2))
        } else {
            0
        }
    }
}

fn clear_rows(layout: &EditorLayout) -> Vec<u16> {
    let mut rows = vec![layout.row];
    if layout.overlay_visible {
        rows.extend(
            (0..layout.overlay_height)
                .map(|offset| layout.overlay_row + offset)
                .filter(|row| *row < layout.area.height),
        );
    }
    rows.sort_unstable();
    rows.dedup();
    rows
}
