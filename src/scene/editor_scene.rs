use super::overlay::{OverlayElement, overlay_width};
use super::{
    Element, Scene,
    editor_layout::EditorLayout,
    elements::{CommandElement, CursorElement, PromptElement},
};
use crate::{
    editor::EditorState,
    input::{CursorPosition, TerminalSize},
};
use ratatui::layout::Rect;
use sopra_completion::CompletionItem;
use std::time::Instant;
pub(super) fn build(editor: &EditorState, size: TerminalSize, now: Instant) -> Scene {
    let items = editor.visible_items().to_vec();
    let item_queries: Vec<String> = items
        .iter()
        .map(|item| editor.completion_query_for(item))
        .collect();
    let prompt = crate::prompt::parse(editor.prompt());
    let rprompt = crate::prompt::parse(editor.rprompt());
    let mut layout = EditorLayout::new(
        editor,
        size,
        crate::prompt::lines::widths(&prompt)
            .last()
            .copied()
            .unwrap_or(0),
        crate::prompt::width(&rprompt),
    );
    layout.footer = editor.completion_footer();
    layout.footer_hint = editor.completion_hint().to_string();
    layout.overlay_width = overlay_width(&items, &layout.footer_hint, &layout.footer, size.columns);
    let elements = editor_elements(editor, &layout, now, items, item_queries, prompt, rprompt);
    let cursor_cells = (0..editor.cursor_cell_width())
        .map(|offset| (layout.cursor_column.saturating_add(offset), layout.row))
        .filter(|(column, _)| *column < size.columns)
        .collect::<Vec<_>>();
    let repaint_cells = if editor.cursor_animation_enabled() {
        cursor_cells
    } else {
        Vec::new()
    };
    Scene::new(
        layout.area,
        clear_rows(&layout),
        repaint_cells,
        CursorPosition {
            column: layout.cursor_column,
            row: layout.row,
        },
        layout.required_height(),
        elements,
    )
}
fn editor_elements(
    editor: &EditorState,
    layout: &EditorLayout,
    now: Instant,
    items: Vec<CompletionItem>,
    item_queries: Vec<String>,
    prompt: Vec<crate::prompt::PromptSpan>,
    rprompt: Vec<crate::prompt::PromptSpan>,
) -> Vec<Box<dyn Element>> {
    let mut elements: Vec<Box<dyn Element>> = vec![
        Box::new(PromptElement {
            column: layout.column,
            row: layout.prompt_row,
            spans: prompt,
        }),
        Box::new(CommandElement {
            column: layout.line_column,
            row: layout.row,
            buffer: editor.buffer().to_string(),
            spans: editor.syntax().to_vec(),
            max_width: if layout.row == layout.prompt_row {
                layout.rprompt_column
            } else {
                layout.area.width
            }
            .saturating_sub(layout.line_column),
        }),
    ];
    if !rprompt.is_empty() && layout.rprompt_column < layout.area.width {
        elements.push(Box::new(PromptElement {
            column: layout.rprompt_column,
            row: layout.prompt_row,
            spans: rprompt,
        }));
    }
    if layout.overlay_height > 0 {
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
            item_queries,
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

fn clear_rows(layout: &EditorLayout) -> Vec<u16> {
    let mut rows: Vec<u16> = (layout.prompt_row..=layout.row).collect();
    if layout.overlay_height > 0 {
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
