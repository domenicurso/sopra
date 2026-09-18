use std::time::Instant;

use ratatui::layout::Rect;
use ratatui::style::Color;

use crate::{
    completion::CompletionItem,
    editor::EditorState,
    input::{CursorPosition, TerminalSize},
};

use super::overlay::{OverlayElement, overlay_width};
use super::{
    Element, Scene,
    elements::{CommandElement, CursorElement, PromptElement},
    overlay_layout,
};

pub(super) fn build(editor: &EditorState, size: TerminalSize, now: Instant) -> Scene {
    let items = editor.visible_items().to_vec();
    let prompt = colored_prompt(editor.prompt());
    let mut layout = EditorLayout::new(editor, size, crate::prompt::width(&prompt));
    layout.footer = editor.completion_footer();
    layout.footer_hint = editor.completion_hint().to_string();
    layout.overlay_width = overlay_width(&items, &layout.footer_hint, &layout.footer, size.columns);
    let elements = editor_elements(editor, &layout, now, items, prompt);
    let cursor_cells = (0..editor.cursor_cell_width())
        .map(|offset| (layout.cursor_column.saturating_add(offset), layout.row))
        .filter(|(column, _)| *column < size.columns)
        .collect();
    Scene::new(
        layout.area,
        clear_rows(&layout),
        cursor_cells,
        CursorPosition {
            column: layout.cursor_column,
            row: layout.row,
        },
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
    completion_token_width: u16,
    footer_hint: String,
    footer: String,
}

impl EditorLayout {
    fn new(editor: &EditorState, size: TerminalSize, prompt_width: usize) -> Self {
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
        let placement = overlay_layout::for_editor(editor, size, row);
        let overlay_visible = placement.is_some();
        let overlay_row = placement.as_ref().map_or(0, |value| value.row);
        let overlay_height = placement.as_ref().map_or(0, |value| value.height);
        let overlay_width = 0;
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
            overlay_above: placement.is_some_and(|value| value.above),
            completion_token_width,
            footer_hint: String::new(),
            footer: String::new(),
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
            viewport_start: editor.suggestion_viewport_start(),
            query: editor.query().to_string(),
            footer_hint: layout.footer_hint.clone(),
            footer: layout.footer.clone(),
            connector_column: layout.connector_column(),
            connector: if layout.overlay_above { "┬" } else { "┴" },
        }));
    }
    elements.push(Box::new(CursorElement {
        column: layout.cursor_column,
        row: layout.row,
        symbol: editor.cursor_symbol(),
        width: editor.cursor_cell_width(),
        style: editor.cursor_style(now),
    }));
    elements
}

fn colored_prompt(prompt: &str) -> Vec<crate::prompt::PromptSpan> {
    crate::prompt::parse(prompt)
        .into_iter()
        .map(|mut span| {
            if span.style.fg.is_none() {
                span.style.fg = Some(Color::LightBlue);
            }
            span
        })
        .collect()
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
