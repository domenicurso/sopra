use std::time::Instant;

use ratatui::style::{Color, Modifier};

use super::Scene;
use crate::{
    editor::{EditorConfig, EditorState},
    input::{CursorPosition, TerminalSize},
};
use sopra_completion::{CompletionItem, CompletionKind};

fn editor(buffer: &str, size: TerminalSize) -> EditorState {
    EditorState::new(EditorConfig {
        buffer: buffer.to_string(),
        cursor_chars: buffer.chars().count(),
        prompt: "$ ".to_string(),
        anchor: CursorPosition { row: 0, column: 0 },
        size,
        cwd: std::path::PathBuf::from("."),
        palette: crate::palette::TerminalPalette::default(),
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
fn overlay_clear_rows_stay_inside_the_terminal() {
    let size = TerminalSize::new(12, 4);
    let mut editor = editor("cd ", size);
    editor.set_completion_source_for_test(
        (0..20)
            .map(|index| {
                CompletionItem::new(
                    format!("entry-{index}"),
                    "",
                    format!("entry-{index}"),
                    CompletionKind::Generic,
                )
            })
            .collect(),
    );
    let frame = Scene::for_editor(&editor, size, Instant::now()).render();
    assert!(frame.clear_rows.iter().all(|row| *row < size.rows));
}

#[test]
fn overlay_cells_keep_the_terminal_background() {
    let size = TerminalSize::new(80, 24);
    let mut editor = editor("", size);
    editor.set_completion_source_for_test(vec![CompletionItem::new(
        "alpha",
        "command",
        "alpha",
        CompletionKind::Generic,
    )]);
    let frame = Scene::for_editor(&editor, size, Instant::now()).render();
    assert_eq!(frame.buffer[(2, 2)].symbol(), "a");
    assert_eq!(frame.buffer[(2, 2)].bg, Color::Reset);
    assert!(frame.buffer[(2, 2)].modifier.contains(Modifier::REVERSED));
    assert_eq!(frame.buffer[(9, 2)].symbol(), "c");
}

#[test]
fn normal_prompt_text_uses_the_prompt_color() {
    let size = TerminalSize::new(80, 24);
    let frame = Scene::for_editor(&editor("", size), size, Instant::now()).render();
    assert_eq!(frame.buffer[(0, 0)].fg, Color::LightBlue);
}

#[test]
fn transient_scene_keeps_the_command_and_its_style() {
    let size = TerminalSize::new(80, 24);
    let editor = editor("print -r -- hi", size);
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
    let editor = editor("echo \"", size);
    let frame = Scene::for_transient(&editor, size).render();
    assert_eq!(frame.buffer[(2, 0)].fg, Color::Green);
    assert_eq!(frame.buffer[(7, 0)].fg, Color::LightRed);
    assert!(frame.buffer[(7, 0)].modifier.contains(Modifier::UNDERLINED));
}

#[test]
fn transient_scene_does_not_keep_cursor_pair_highlighting() {
    let size = TerminalSize::new(80, 24);
    let editor = EditorState::new(EditorConfig {
        buffer: "echo \"hi\"".to_string(),
        cursor_chars: 6,
        prompt: "$ ".to_string(),
        anchor: CursorPosition { row: 0, column: 0 },
        size,
        cwd: std::path::PathBuf::from("."),
        palette: crate::palette::TerminalPalette::default(),
    });
    let frame = Scene::for_transient(&editor, size).render();
    assert_eq!(frame.buffer[(7, 0)].fg, Color::LightMagenta);
    assert!(!frame.buffer[(7, 0)].modifier.contains(Modifier::UNDERLINED));
    assert!(
        !frame.buffer[(10, 0)]
            .modifier
            .contains(Modifier::UNDERLINED)
    );
}

#[test]
fn cursor_repaint_covers_a_wide_grapheme() {
    let size = TerminalSize::new(80, 24);
    let editor = EditorState::new(EditorConfig {
        buffer: "💡".to_string(),
        cursor_chars: 0,
        prompt: "$ ".to_string(),
        anchor: CursorPosition { row: 0, column: 0 },
        size,
        cwd: std::path::PathBuf::from("."),
        palette: crate::palette::TerminalPalette::default(),
    });
    let frame = Scene::for_editor(&editor, size, Instant::now()).render();
    assert_eq!(frame.repaint_cells, vec![(2, 0), (3, 0)]);
}

#[test]
fn cursor_repaints_the_character_under_it() {
    let size = TerminalSize::new(80, 24);
    let editor = EditorState::new(EditorConfig {
        buffer: "echo".to_string(),
        cursor_chars: 1,
        prompt: "$ ".to_string(),
        anchor: CursorPosition { row: 0, column: 0 },
        size,
        cwd: std::path::PathBuf::from("."),
        palette: crate::palette::TerminalPalette::default(),
    });
    let frame = Scene::for_editor(&editor, size, Instant::now()).render();
    assert_eq!(frame.buffer[(3, 0)].symbol(), "c");
}
