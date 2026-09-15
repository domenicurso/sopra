mod canvas;
mod editor_scene;
mod elements;
mod overlay;
mod transient;

#[cfg(test)]
mod tests;

use std::time::Instant;

use ratatui::{buffer::Buffer, layout::Rect};

use crate::{editor::EditorState, input::TerminalSize};

use canvas::Canvas;

const MAX_OVERLAY_ITEMS: usize = 6;
const MAX_OVERLAY_WIDTH: u16 = 48;
const OVERLAY_FOOTER: &str = "↑↓ select · Tab · Enter · Esc";
pub(crate) const TRANSIENT_PROMPT: &str = "❯ ";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DemoItemKind {
    Command,
    File,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DemoItem {
    pub(crate) label: &'static str,
    pub(crate) kind: DemoItemKind,
}

const DEMO_ITEMS: &[DemoItem] = &[
    DemoItem {
        label: "git status",
        kind: DemoItemKind::Command,
    },
    DemoItem {
        label: "git switch -c demo",
        kind: DemoItemKind::Command,
    },
    DemoItem {
        label: "cargo test",
        kind: DemoItemKind::Command,
    },
    DemoItem {
        label: "README.md",
        kind: DemoItemKind::File,
    },
    DemoItem {
        label: "src/",
        kind: DemoItemKind::File,
    },
    DemoItem {
        label: "keel --help",
        kind: DemoItemKind::Help,
    },
];

pub(crate) fn demo_items(query: &str) -> Vec<DemoItem> {
    let query = query.to_ascii_lowercase();
    let mut matches: Vec<_> = DEMO_ITEMS
        .iter()
        .copied()
        .filter(|item| query.is_empty() || item.label.to_ascii_lowercase().contains(&query))
        .collect();
    if matches.is_empty() {
        matches.push(DemoItem {
            label: "no matches",
            kind: DemoItemKind::Help,
        });
    }
    matches
}

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
