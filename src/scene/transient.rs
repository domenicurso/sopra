use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
};

use crate::{
    editor::EditorState,
    input::{CursorPosition, TerminalSize},
};

use super::{
    Element, Scene, TRANSIENT_PROMPT,
    elements::{CommandElement, PromptElement},
};

pub(super) fn build(editor: &EditorState, size: TerminalSize) -> Scene {
    let area = Rect::new(0, 0, size.columns, size.rows);
    let elements: Vec<Box<dyn Element>> = vec![
        Box::new({
            let mut prompt = PromptElement::styled(
                TRANSIENT_PROMPT,
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM),
            );
            prompt.column = 0;
            prompt.row = 0;
            prompt
        }),
        Box::new(CommandElement {
            column: 2,
            row: 0,
            buffer: editor.buffer().to_string(),
            spans: editor.transient_syntax(),
        }),
    ];
    Scene::new(
        area,
        vec![0],
        Vec::new(),
        CursorPosition { column: 0, row: 0 },
        elements,
    )
}
