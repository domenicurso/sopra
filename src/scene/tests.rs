use std::time::Instant;

use ratatui::style::Color;

use super::Scene;
use crate::{
    editor::{EditorConfig, EditorState},
    input::{CursorPosition, TerminalSize},
};

fn editor(buffer: &str, size: TerminalSize) -> EditorState {
    EditorState::new(EditorConfig {
        buffer: buffer.to_string(),
        cursor_chars: buffer.chars().count(),
        prompt: "keel-demo ❯ ".to_string(),
        anchor: CursorPosition { row: 0, column: 0 },
        size,
    })
}

#[test]
fn editor_scene_is_bounded_for_a_tiny_terminal() {
    let size = TerminalSize::new(1, 1);
    let frame = Scene::for_editor(&editor("", size), size, Instant::now()).render();
    assert_eq!(frame.buffer.area.width, 1);
    assert_eq!(frame.buffer.area.height, 1);
}

#[test]
fn overlay_cells_keep_the_terminal_background() {
    let size = TerminalSize::new(80, 24);
    let frame = Scene::for_editor(&editor("", size), size, Instant::now()).render();
    assert_eq!(frame.buffer[(40, 3)].bg, Color::Reset);
}

#[test]
fn transient_scene_keeps_the_command_and_its_style() {
    let size = TerminalSize::new(80, 24);
    let frame = Scene::for_transient(&editor("print -r -- hi", size), size).render();
    assert_eq!(frame.buffer[(0, 0)].symbol(), "❯");
    assert_eq!(frame.buffer[(2, 0)].symbol(), "p");
    assert_eq!(frame.buffer[(2, 0)].fg, Color::Rgb(125, 196, 255));
    assert_eq!(frame.buffer[(30, 0)], ratatui::buffer::Cell::EMPTY);
}
