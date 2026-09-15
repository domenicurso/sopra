use std::time::Instant;

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
};

use crate::{editor::EditorState, input::TerminalSize};

use super::overlay::{OverlayElement, overlay_width};
use super::{
    Element, MAX_OVERLAY_ITEMS, Scene,
    elements::{CommandElement, CursorElement, PromptElement, StatusElement, display_width},
};

pub(super) fn build(editor: &EditorState, size: TerminalSize, now: Instant) -> Scene {
    let items = editor.visible_items();
    let layout = EditorLayout::new(editor, size, &items);
    let elements = editor_elements(editor, &layout, now, items);
    Scene::new(
        layout.area,
        clear_rows(&layout),
        vec![(layout.cursor_column, layout.row)],
        elements,
    )
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
    status_row: u16,
}

impl EditorLayout {
    fn new(editor: &EditorState, size: TerminalSize, items: &[super::DemoItem]) -> Self {
        let area = Rect::new(0, 0, size.columns, size.rows);
        let row = editor.anchor().row.min(size.rows.saturating_sub(1));
        let column = editor.anchor().column.min(size.columns.saturating_sub(1));
        let line_column = column
            .saturating_add(display_width(editor.prompt()) as u16)
            .min(size.columns.saturating_sub(1));
        let cursor_column = line_column
            .saturating_add(editor.cursor_display_width())
            .min(size.columns.saturating_sub(1));
        let overlay_visible = editor.overlay_visible();
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
        let status_row = if below_fits {
            overlay_row
                .saturating_add(overlay_height)
                .min(size.rows.saturating_sub(1))
        } else {
            size.rows.saturating_sub(1)
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
            status_row,
        }
    }
}

fn editor_elements(
    editor: &EditorState,
    layout: &EditorLayout,
    now: Instant,
    items: Vec<super::DemoItem>,
) -> Vec<Box<dyn Element>> {
    let mut elements: Vec<Box<dyn Element>> = vec![
        Box::new(PromptElement {
            column: layout.column,
            row: layout.row,
            text: editor.prompt().to_string(),
            style: Style::default()
                .fg(Color::Rgb(111, 214, 176))
                .add_modifier(Modifier::BOLD),
        }),
        Box::new(CommandElement {
            column: layout.line_column,
            row: layout.row,
            buffer: editor.buffer().to_string(),
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
            query: editor.query().to_string(),
            connector_column: layout.connector_column(),
            connector: if layout.overlay_above { "┬" } else { "┴" },
        }));
    }
    elements.push(Box::new(StatusElement {
        column: layout.column,
        row: layout.status_row,
        width: layout.area.width.saturating_sub(layout.column),
        text: editor.status_text(),
    }));
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
        self.line_column
            .saturating_sub(2)
            .min(self.area.width.saturating_sub(self.overlay_width))
    }

    fn connector_column(&self) -> u16 {
        if self.overlay_width >= 3 {
            self.line_column
                .saturating_sub(self.overlay_column())
                .clamp(1, self.overlay_width.saturating_sub(2))
        } else {
            0
        }
    }
}

fn clear_rows(layout: &EditorLayout) -> Vec<u16> {
    let mut rows = vec![layout.row, layout.status_row];
    if layout.overlay_visible {
        rows.extend((0..layout.overlay_height).map(|offset| layout.overlay_row + offset));
    }
    rows.sort_unstable();
    rows.dedup();
    rows
}
