use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
};

use crate::{editor::EditorState, input::TerminalSize};

use super::{
    Element, Scene, TRANSIENT_PROMPT,
    elements::{CommandElement, PromptElement},
};

pub(super) fn build(editor: &EditorState, size: TerminalSize) -> Scene {
    let area = Rect::new(0, 0, size.columns, size.rows);
    let elements: Vec<Box<dyn Element>> = vec![
        Box::new(PromptElement {
            column: 0,
            row: 0,
            text: TRANSIENT_PROMPT.to_string(),
            style: Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM),
        }),
        Box::new(CommandElement {
            column: 2,
            row: 0,
            buffer: editor.buffer().to_string(),
            spans: editor.syntax().to_vec(),
        }),
    ];
    Scene::new(area, vec![0], Vec::new(), elements)
}
