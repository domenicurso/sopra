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
    let width = (crate::prompt::width(&spans) as u16).min(size.columns);
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
            column: width,
            row: 0,
            buffer: editor.buffer().to_string(),
            spans: editor.transient_syntax(),
            selection: None,
            max_width: size.columns.saturating_sub(width),
        }),
    ];
    Scene::new(
        area,
        vec![0],
        Vec::new(),
        CursorPosition { column: 0, row: 0 },
        1,
        elements,
    )
}
