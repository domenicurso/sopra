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
fn editor_scene_reports_the_popup_rows_that_must_fit_below_the_prompt() {
    let size = TerminalSize::new(80, 24);
    let mut editor = editor("cd ", size);
    editor.set_completion_source_for_test(vec![CompletionItem::new(
        "alpha",
        "command",
        "alpha",
        CompletionKind::Generic,
    )]);
    let frame = Scene::for_editor(&editor, size, Instant::now()).render();
    assert_eq!(frame.required_height, 4);
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
    assert!(!frame.buffer[(2, 2)].modifier.contains(Modifier::REVERSED));
    assert_eq!(frame.buffer[(9, 2)].symbol(), "c");
}

#[test]
fn normal_prompt_text_uses_terminal_default_color() {
    let size = TerminalSize::new(80, 24);
    let frame = Scene::for_editor(&editor("", size), size, Instant::now()).render();
    assert_eq!(frame.buffer[(0, 0)].fg, Color::Reset);
}

#[test]
fn cursor_repaint_covers_a_wide_grapheme() {
    let size = TerminalSize::new(80, 24);
    let editor = EditorState::new(EditorConfig {
        buffer: "💡".to_string(),
        cursor_chars: 0,
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
        palette: crate::palette::TerminalPalette::default(),
    });
    let frame = Scene::for_editor(&editor, size, Instant::now()).render();
    assert_eq!(frame.repaint_cells, vec![(2, 0), (3, 0)]);
}

#[test]
fn disabled_cursor_animation_does_not_force_repaints() {
    let size = TerminalSize::new(80, 24);
    let editor = EditorState::new(EditorConfig {
        buffer: "echo".to_string(),
        cursor_chars: 4,
        arm_completion: false,
        suppress_completion: false,
        completion_enabled: true,
        syntax_highlighting: true,
        cursor_animation: false,
        prompt: "$ ".to_string(),
        rprompt: String::new(),
        anchor: CursorPosition { row: 0, column: 0 },
        size,
        cwd: std::path::PathBuf::from("."),
        palette: crate::palette::TerminalPalette::default(),
    });
    let frame = Scene::for_editor(&editor, size, Instant::now()).render();
    assert!(frame.repaint_cells.is_empty());
}

#[test]
fn cursor_repaints_the_character_under_it() {
    let size = TerminalSize::new(80, 24);
    let editor = EditorState::new(EditorConfig {
        buffer: "echo".to_string(),
        cursor_chars: 1,
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
        palette: crate::palette::TerminalPalette::default(),
    });
    let frame = Scene::for_editor(&editor, size, Instant::now()).render();
    assert_eq!(frame.buffer[(3, 0)].symbol(), "c");
}
