mod canvas;
mod editor_scene;
mod elements;
mod overlay;
mod overlay_items;
mod transient;

#[cfg(test)]
mod tests;

use std::time::Instant;

use ratatui::{buffer::Buffer, layout::Rect};

use crate::{editor::EditorState, input::TerminalSize};

use canvas::Canvas;

const MAX_OVERLAY_ITEMS: usize = 12;
const MAX_OVERLAY_WIDTH: u16 = 56;
const OVERLAY_FOOTER: &str = "↑↓ · Tab · Esc";
pub(crate) const TRANSIENT_PROMPT: &str = "❯ ";

pub(crate) struct Frame {
    pub(crate) buffer: Buffer,
    pub(crate) clear_rows: Vec<u16>,
    pub(crate) repaint_cells: Vec<(u16, u16)>,
}

trait Element {
    fn paint(&self, canvas: &mut Canvas);
}

pub(crate) struct Scene {
    area: Rect,
    clear_rows: Vec<u16>,
    repaint_cells: Vec<(u16, u16)>,
    elements: Vec<Box<dyn Element>>,
}

impl Scene {
    pub(crate) fn for_editor(editor: &EditorState, size: TerminalSize, now: Instant) -> Self {
        editor_scene::build(editor, size, now)
    }

    pub(crate) fn for_transient(editor: &EditorState, size: TerminalSize) -> Self {
        transient::build(editor, size)
    }

    fn new(
        area: Rect,
        clear_rows: Vec<u16>,
        repaint_cells: Vec<(u16, u16)>,
        elements: Vec<Box<dyn Element>>,
    ) -> Self {
        Self {
            area,
            clear_rows,
            repaint_cells,
            elements,
        }
    }

    pub(crate) fn render(self) -> Frame {
        let mut canvas = Canvas::new(self.area);
        for element in self.elements {
            element.paint(&mut canvas);
        }
        Frame {
            buffer: canvas.into_buffer(),
            clear_rows: self.clear_rows,
            repaint_cells: self.repaint_cells,
        }
    }
}
