use std::time::Instant;

use ratatui::style::{Color, Modifier};

use crate::{
    editor::{EditorConfig, EditorState},
    input::{CursorPosition, TerminalSize},
    palette::TerminalPalette,
};

use super::Scene;

fn editor(buffer: &str, cursor_chars: usize, size: TerminalSize) -> EditorState {
    EditorState::new(EditorConfig {
        buffer: buffer.to_string(),
        cursor_chars,
        arm_completion: false,
        suppress_completion: false,
        completion_enabled: true,
        syntax_highlighting: true,
        cursor_animation: true,
        prompt: "$ ".to_string(),
        rprompt: String::new(),
        anchor: CursorPosition { row: 0, column: 0 },
        size,
        cwd: std::path::PathBuf::from("."),
        palette: TerminalPalette::default(),
    })
}

#[test]
fn transient_scene_keeps_the_command_and_its_style() {
    let size = TerminalSize::new(80, 24);
    let buffer = "print -r -- hi";
    let editor = editor(buffer, buffer.chars().count(), size);
    let frame = Scene::for_transient(&editor, size).render();
    let live = Scene::for_editor(&editor, size, Instant::now()).render();
    assert_eq!(frame.buffer[(0, 0)].symbol(), "$");
    assert_eq!(frame.buffer[(2, 0)].symbol(), "p");
    assert_eq!(frame.buffer[(2, 0)].fg, Color::Green);
    assert_eq!(frame.buffer[(2, 0)].fg, live.buffer[(2, 0)].fg);
    assert_eq!(frame.buffer[(2, 0)].modifier, live.buffer[(2, 0)].modifier);
    assert_eq!(frame.buffer[(30, 0)], ratatui::buffer::Cell::EMPTY);
}

#[test]
fn transient_scene_keeps_partial_syntax_highlighting() {
    let size = TerminalSize::new(80, 24);
    let buffer = "echo \"";
    let editor = editor(buffer, buffer.chars().count(), size);
    let frame = Scene::for_transient(&editor, size).render();
    assert_eq!(frame.buffer[(2, 0)].fg, Color::Green);
    assert_eq!(frame.buffer[(7, 0)].fg, Color::LightRed);
    assert!(frame.buffer[(7, 0)].modifier.contains(Modifier::UNDERLINED));
}

#[test]
fn transient_scene_does_not_keep_cursor_pair_highlighting() {
    let size = TerminalSize::new(80, 24);
    let editor = editor("echo \"hi\"", 6, size);
    let frame = Scene::for_transient(&editor, size).render();
    assert_eq!(frame.buffer[(7, 0)].fg, Color::LightMagenta);
    assert!(!frame.buffer[(7, 0)].modifier.contains(Modifier::UNDERLINED));
    assert!(
        !frame.buffer[(10, 0)]
            .modifier
            .contains(Modifier::UNDERLINED)
    );
}
