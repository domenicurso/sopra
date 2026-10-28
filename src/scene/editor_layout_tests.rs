use super::EditorLayout;
use crate::{
    editor::{EditorConfig, EditorState},
    input::{CursorPosition, TerminalSize},
    palette::TerminalPalette,
};

fn editor(cursor_chars: usize) -> EditorState {
    editor_with_buffer(33, cursor_chars)
}

fn editor_with_buffer(buffer_chars: usize, cursor_chars: usize) -> EditorState {
    let buffer = "1".repeat(buffer_chars);
    EditorState::new(EditorConfig {
        buffer,
        cursor_chars,
        arm_completion: false,
        suppress_completion: true,
        completion_enabled: false,
        syntax_highlighting: false,
        cursor_animation: false,
        prompt: "$ ".to_string(),
        rprompt: "RIGHT".to_string(),
        anchor: CursorPosition { row: 0, column: 0 },
        size: TerminalSize::new(40, 24),
        cwd: std::path::PathBuf::new(),
        palette: TerminalPalette::default(),
    })
}

#[test]
fn multiline_input_starts_at_the_left_edge_after_a_nonzero_anchor() {
    let editor = EditorState::new(EditorConfig {
        buffer: "abc".into(),
        cursor_chars: 3,
        arm_completion: false,
        suppress_completion: true,
        completion_enabled: false,
        syntax_highlighting: false,
        cursor_animation: false,
        prompt: "first\n$ ".into(),
        rprompt: "right".into(),
        anchor: CursorPosition { row: 0, column: 7 },
        size: TerminalSize::new(40, 24),
        cwd: std::path::PathBuf::new(),
        palette: TerminalPalette::default(),
    });
    let layout = EditorLayout::new(&editor, TerminalSize::new(40, 24), 2, 5);
    assert_eq!(
        (
            layout.prompt_row,
            layout.row,
            layout.line_column,
            layout.cursor_column
        ),
        (0, 1, 2, 5)
    );
}

#[test]
fn right_prompt_does_not_cover_the_end_cursor_at_an_exact_fit() {
    let size = TerminalSize::new(40, 24);
    let layout = EditorLayout::new(&editor(33), size, 2, 5);

    assert_eq!(layout.rprompt_column, size.columns);
}

#[test]
fn right_prompt_ends_at_the_terminal_edge_when_there_is_room() {
    let layout = EditorLayout::new(&editor_with_buffer(29, 29), TerminalSize::new(40, 24), 2, 5);

    assert_eq!(layout.rprompt_column, 35);
}

#[test]
fn right_prompt_is_hidden_when_the_terminal_has_no_safe_right_edge() {
    let layout = EditorLayout::new(&editor(0), TerminalSize::new(1, 24), 2, 5);

    assert_eq!(layout.rprompt_column, 1);
}
