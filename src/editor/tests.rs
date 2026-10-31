use super::{EditorConfig, EditorState, ExitReason};
use crate::{
    actions::Action,
    input::{CursorPosition, TerminalSize},
};

fn editor(buffer: &str) -> EditorState {
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
        size: TerminalSize::new(80, 24),
        cwd: std::path::PathBuf::from("."),
        palette: crate::palette::TerminalPalette::default(),
    })
}

#[path = "completion_acceptance_tests.rs"]
mod acceptance;
#[path = "completion_lifecycle_tests.rs"]
mod lifecycle;

#[test]
fn cursor_input_is_character_based_but_state_is_byte_safe() {
    let mut state = editor("a💡b");
    state.handle_key(Action::Home);
    state.handle_key(Action::Right);
    state.handle_key(Action::Character('x'));
    assert_eq!(state.buffer(), "ax💡b");
}

#[test]
fn grapheme_navigation_preserves_emoji() {
    let mut state = editor("a👩‍💻b");
    state.handle_key(Action::Backspace);
    assert_eq!(state.buffer(), "a👩‍💻");
    state.handle_key(Action::Backspace);
    assert_eq!(state.buffer(), "a");
}

#[test]
fn escape_keeps_the_edited_buffer() {
    let mut state = editor("");
    state.handle_key(Action::Character('g'));
    state.handle_key(Action::Character('i'));
    assert!(state.handle_key(Action::Escape).is_none());
    assert_eq!(state.buffer(), "gi");
    assert!(!state.overlay_visible());
    state.handle_key(Action::Character('t'));
    assert!(state.overlay_visible());
}

#[test]
fn accepted_result_carries_cursor_free_syntax_highlighting() {
    let state = editor("echo \"hi\"");
    let result = state.result(ExitReason::Accepted);

    assert!(
        result
            .highlights
            .iter()
            .any(|span| span.kind == crate::syntax::SyntaxKind::RealCommand)
    );
    assert!(
        result
            .highlights
            .iter()
            .all(|span| span.end <= result.buffer.len())
    );
}

#[test]
fn paired_quotes_and_brackets_are_inserted_and_skipped() {
    let mut state = editor("");
    state.handle_key(Action::Character('"'));
    assert_eq!(state.buffer(), "\"\"");
    state.handle_key(Action::Character('"'));
    assert_eq!(state.buffer(), "\"\"");
    let mut brackets = editor("");
    brackets.handle_key(Action::Character('('));
    assert_eq!(brackets.buffer(), "()");
    brackets.handle_key(Action::Character(')'));
    assert_eq!(brackets.buffer(), "()");
}

#[test]
fn comments_do_not_receive_auto_pairs() {
    let mut state = editor("# ");
    state.handle_key(Action::Character('('));
    assert_eq!(state.buffer(), "# (");
}

#[test]
fn eof_deletes_a_character_and_delegates_on_an_empty_line() {
    let mut state = editor("git st");
    state.handle_key(Action::Home);
    state.handle_key(Action::Eof);
    assert_eq!(state.buffer(), "it st");

    let result = editor("").handle_key(Action::Eof).expect("EOF result");
    assert_eq!(result.reason, ExitReason::DelegateEof);
}

#[test]
fn empty_history_navigation_keeps_the_buffer_and_tab_can_delegate() {
    let mut state = editor("");
    assert!(state.handle_key(Action::Up).is_none());
    assert!(state.handle_key(Action::Down).is_none());
    assert_eq!(state.buffer(), "");
    assert_eq!(
        editor("")
            .handle_key(Action::Tab)
            .expect("completion result")
            .reason,
        ExitReason::DelegateTab
    );
}

#[test]
fn word_editing_and_yank_keep_the_buffer_byte_safe() {
    let mut state = editor("git status");
    state.handle_key(Action::WordBackspace);
    assert_eq!(state.buffer(), "git ");
    state.handle_key(Action::Yank);
    assert_eq!(state.buffer(), "git status");
    state.handle_key(Action::KillToStart);
    assert_eq!(state.buffer(), "");
    state.handle_key(Action::Yank);
    assert_eq!(state.buffer(), "git status");
}

#[test]
fn paste_replaces_selected_text_without_pairing_or_submitting_newlines() {
    use crate::input::MouseKind;
    let mut editor = editor("hello world");
    let now = std::time::Instant::now();
    editor
        .text_selection
        .mouse(MouseKind::Press, 6, &editor.buffer, now);
    editor
        .text_selection
        .mouse(MouseKind::Drag, 11, &editor.buffer, now);
    editor.insert_paste("'é'\nnext(");
    assert_eq!(editor.buffer, "hello 'é'\nnext(");
    assert_eq!(editor.cursor, editor.buffer.len());
    assert!(editor.text_selection.range().is_none());
}
