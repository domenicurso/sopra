use ratatui::layout::Rect;

use crate::{
    editor::EditorState,
    input::{CursorPosition, TerminalSize},
};

use super::{
    Element, Scene,
    elements::{CommandElement, PromptElement},
};

pub(super) fn build(editor: &EditorState, size: TerminalSize) -> Scene {
    let area = Rect::new(0, 0, size.columns, size.rows);
    let spans = crate::prompt::parse(editor.transient_prompt());
    let mut flow =
        super::text_flow::TextFlow::new(CursorPosition { column: 0, row: 0 }, size.columns);
    flow.prompt(&spans);
    let start = flow.cursor();
    flow.command(
        editor.buffer(),
        &crate::prompt::parse(editor.secondary_prompt()),
    );
    let height = flow.cursor().row.saturating_add(1);
    let elements: Vec<Box<dyn Element>> = vec![
        Box::new(PromptElement {
            column: 0,
            row: 0,
            spans: if editor.transient_prompt() == "$ " {
                crate::prompt::parse("\x1b[2m$ ")
            } else {
                spans
            },
        }),
        Box::new(CommandElement {
            column: start.column,
            row: start.row,
            buffer: editor.buffer().to_string(),
            spans: editor.transient_syntax(),
            selection: None,
            secondary: crate::prompt::parse(editor.secondary_prompt()),
        }),
    ];
    Scene::new(
        area,
        (0..height.min(size.rows)).collect(),
        Vec::new(),
        flow.cursor(),
        height,
        elements,
    )
}
