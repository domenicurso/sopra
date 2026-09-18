use super::{EditorConfig, EditorState, ExitReason};
use crate::input::{CursorPosition, Key, TerminalSize};

fn editor(buffer: &str) -> EditorState {
    EditorState::new(EditorConfig {
        buffer: buffer.to_string(),
        cursor_chars: buffer.chars().count(),
        prompt: "❯ ".to_string(),
        anchor: CursorPosition { row: 0, column: 0 },
        size: TerminalSize::new(80, 24),
        cwd: std::path::PathBuf::from("."),
        palette: crate::palette::TerminalPalette::default(),
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
    assert!(state.handle_key(Key::Escape).is_none());
    assert_eq!(state.buffer(), "gi");
    assert!(!state.overlay_visible());
    state.handle_key(Key::Character('t'));
    assert!(state.overlay_visible());
}

#[test]
fn eof_deletes_a_character_and_delegates_on_an_empty_line() {
    let mut state = editor("git st");
    state.handle_key(Key::Home);
    state.handle_key(Key::Eof);
    assert_eq!(state.buffer(), "it st");

    let result = editor("").handle_key(Key::Eof).expect("EOF result");
    assert_eq!(result.reason, ExitReason::DelegateEof);
}

#[test]
fn navigation_and_tab_delegate_when_no_overlay_items_exist() {
    let mut state = editor("");
    assert_eq!(
        state.handle_key(Key::Up).expect("history result").reason,
        ExitReason::DelegateUp
    );
    assert_eq!(
        editor("")
            .handle_key(Key::Down)
            .expect("history result")
            .reason,
        ExitReason::DelegateDown
    );
    assert_eq!(
        editor("")
            .handle_key(Key::Tab)
            .expect("completion result")
            .reason,
        ExitReason::DelegateTab
    );
}

#[test]
fn word_editing_and_yank_keep_the_buffer_byte_safe() {
    let mut state = editor("git status");
    state.handle_key(Key::WordBackspace);
    assert_eq!(state.buffer(), "git ");
    state.handle_key(Key::Yank);
    assert_eq!(state.buffer(), "git status");
    state.handle_key(Key::KillToStart);
    assert_eq!(state.buffer(), "");
    state.handle_key(Key::Yank);
    assert_eq!(state.buffer(), "git status");
}
