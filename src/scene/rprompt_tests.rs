use std::time::Instant;

use ratatui::style::Color;

use crate::{
    editor::{EditorConfig, EditorState},
    input::{CursorPosition, TerminalSize},
    palette::TerminalPalette,
};

use super::Scene;

fn editor(buffer: &str, size: TerminalSize, rprompt: &str) -> EditorState {
    EditorState::new(EditorConfig {
        buffer: buffer.to_string(),
        cursor_chars: buffer.chars().count(),
        arm_completion: false,
        suppress_completion: false,
        completion_enabled: true,
        syntax_highlighting: true,
        cursor_animation: true,
        prompt: "$ ".to_string(),
        rprompt: rprompt.to_string(),
        anchor: CursorPosition { row: 0, column: 0 },
        size,
        cwd: std::path::PathBuf::from("."),
        palette: TerminalPalette::default(),
    })
}

#[test]
fn right_prompt_keeps_ansi_style_and_ends_at_the_terminal_edge() {
    let size = TerminalSize::new(80, 24);
    let editor = editor("", size, "\x1b[32mRIGHT\x1b[0m");
    let frame = Scene::for_editor(&editor, size, Instant::now()).render();
    assert_eq!(frame.buffer[(75, 0)].symbol(), "R");
    assert_eq!(frame.buffer[(75, 0)].fg, Color::Green);
    assert_eq!(frame.buffer[(79, 0)].symbol(), "T");
}

#[test]
fn right_prompt_hides_when_the_command_would_reach_it() {
    let size = TerminalSize::new(40, 24);
    let buffer = "123456789012345678901234567890123456";
    let frame = Scene::for_editor(&editor(buffer, size, "RIGHT"), size, Instant::now()).render();
    let line = (0..size.columns)
        .map(|column| frame.buffer[(column, 0)].symbol())
        .collect::<String>();
    assert!(!line.contains("RIGHT"));
}
