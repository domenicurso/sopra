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
        crate::text::flow::TextFlow::new(CursorPosition { column: 0, row: 0 }, size.columns);
    flow.prompt(&spans);
    flow.command(
        editor.buffer(),
        &crate::prompt::parse(editor.secondary_prompt()),
    );
    let height = flow.cursor().row.saturating_add(1);
    let elements: Vec<Box<dyn Element>> = vec![
        Box::new(PromptElement {
            column: 0,
            row: 0,
            spans: spans.clone(),
        }),
        Box::new(CommandElement {
            layout: std::rc::Rc::new(crate::text::layout::Layout::new(
                crate::text::layout::Input {
                    anchor: CursorPosition { column: 0, row: 0 },
                    columns: size.columns,
                    prompt: &spans,
                    secondary: &crate::prompt::parse(editor.secondary_prompt()),
                    buffer: editor.buffer(),
                },
            )),
            spans: editor.transient_syntax(),
            selection: None,
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
