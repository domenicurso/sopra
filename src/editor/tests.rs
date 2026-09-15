use super::{EditorConfig, EditorState, ExitReason};
use crate::input::{CursorPosition, Key, TerminalSize};

fn editor(buffer: &str) -> EditorState {
    EditorState::new(EditorConfig {
        buffer: buffer.to_string(),
        cursor_chars: buffer.chars().count(),
        prompt: "❯ ".to_string(),
        anchor: CursorPosition { row: 0, column: 0 },
        size: TerminalSize::new(80, 24),
    })
}

#[test]
fn cursor_input_is_character_based_but_state_is_byte_safe() {
    let mut state = editor("a💡b");
    state.handle_key(Key::Home);
    state.handle_key(Key::Right);
    state.handle_key(Key::Character('x'));
    assert_eq!(state.buffer(), "ax💡b");
}

#[test]
fn grapheme_navigation_preserves_emoji() {
    let mut state = editor("a👩‍💻b");
    state.handle_key(Key::Backspace);
    assert_eq!(state.buffer(), "a👩‍💻");
    state.handle_key(Key::Backspace);
    assert_eq!(state.buffer(), "a");
}

#[test]
fn escape_keeps_the_edited_buffer() {
    let mut state = editor("");
    state.handle_key(Key::Character('g'));
    state.handle_key(Key::Character('i'));
    state.handle_key(Key::Escape);
    assert_eq!(state.buffer(), "gi");
}

#[test]
fn eof_restores_the_original_buffer() {
    let mut state = editor("git st");
    state.handle_key(Key::Character('x'));
    let result = state.handle_key(Key::Eof).expect("EOF result");
    assert_eq!(result.reason, ExitReason::Cancelled);
    assert_eq!(result.buffer, "git st");
}
