use super::{EditorConfig, EditorState, ExitReason};
use crate::input::{CursorPosition, Key, TerminalSize};

fn editor(buffer: &str) -> EditorState {
    EditorState::new(EditorConfig {
        buffer: buffer.to_string(),
        cursor_chars: buffer.chars().count(),
        arm_completion: false,
        suppress_completion: false,
        prompt: "$ ".to_string(),
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
fn whitespace_only_input_keeps_the_completion_menu_empty() {
    let state = editor("   ");
    assert!(state.visible_items().is_empty());
}

#[test]
fn paired_quotes_and_brackets_are_inserted_and_skipped() {
    let mut state = editor("");
    state.handle_key(Key::Character('"'));
    assert_eq!(state.buffer(), "\"\"");
    state.handle_key(Key::Character('"'));
    assert_eq!(state.buffer(), "\"\"");
    let mut brackets = editor("");
    brackets.handle_key(Key::Character('('));
    assert_eq!(brackets.buffer(), "()");
    brackets.handle_key(Key::Character(')'));
    assert_eq!(brackets.buffer(), "()");
}

#[test]
fn comments_do_not_receive_auto_pairs() {
    let mut state = editor("# ");
    state.handle_key(Key::Character('('));
    assert_eq!(state.buffer(), "# (");
}

#[test]
fn structured_completion_can_leave_the_cursor_inside_a_pair() {
    let mut state = editor("export ar");
    let item = sopra_completion::CompletionItem::new(
        "arr",
        "shell array",
        "arr",
        sopra_completion::CompletionKind::Array,
    )
    .with_suffix("=()", 4);
    state.set_completion_source_for_test(vec![item]);
    state.handle_key(Key::Tab);
    assert_eq!(state.buffer(), "export arr=()");
    assert_eq!(state.result(ExitReason::Accepted).cursor, 11);
}

#[test]
fn attached_value_completion_preserves_the_assignment_key() {
    let mut state = editor("npm access set mfa=");
    let start = "npm access set mfa=".len();
    let item = sopra_completion::CompletionItem::with_range(
        "automation",
        "automation",
        None,
        sopra_completion::CompletionKind::Value,
        start..start,
        sopra_completion::CompletionSource::CommandIndex,
    );
    state.set_completion_source_for_test(vec![item]);

    state.handle_key(Key::Tab);
    assert_eq!(state.buffer(), "npm access set mfa=automation ");
}

#[test]
fn accepting_a_word_completion_appends_a_space() {
    let mut state = editor("git st");
    let end = state.buffer().len();
    state.set_completion_source_for_test(vec![sopra_completion::CompletionItem::with_range(
        "status",
        "status",
        None,
        sopra_completion::CompletionKind::Subcommand,
        4..end,
        sopra_completion::CompletionSource::CommandIndex,
    )]);

    state.handle_key(Key::Tab);

    assert_eq!(state.buffer(), "git status ");
}

#[test]
fn accepting_an_assignment_key_keeps_the_value_attached() {
    let mut state = editor("npm access set ");
    let end = state.buffer().len();
    state.set_completion_source_for_test(vec![sopra_completion::CompletionItem::with_range(
        "mfa=",
        "mfa=",
        None,
        sopra_completion::CompletionKind::Value,
        end..end,
        sopra_completion::CompletionSource::CommandIndex,
    )]);

    state.handle_key(Key::Tab);

    assert_eq!(state.buffer(), "npm access set mfa=");
}

#[test]
fn suspended_history_completion_waits_for_editing() {
    let mut state = EditorState::new(EditorConfig {
        buffer: "git st".to_string(),
        cursor_chars: 6,
        arm_completion: false,
        suppress_completion: true,
        prompt: "$ ".to_string(),
        anchor: CursorPosition { row: 0, column: 0 },
        size: TerminalSize::new(80, 24),
        cwd: std::path::PathBuf::from("."),
        palette: crate::palette::TerminalPalette::default(),
    });
    state.set_completion_source_for_test(vec![sopra_completion::CompletionItem::new(
        "status",
        "",
        "status",
        sopra_completion::CompletionKind::Subcommand,
    )]);

    assert!(state.visible_items().is_empty());
    assert!(!state.overlay_visible());

    state.handle_key(Key::Character('x'));

    assert!(!state.completion_suspended);
    assert!(state.overlay_visible());
}

#[test]
fn tab_focuses_the_first_completion_before_accepting_it() {
    let mut state = editor("git ");
    state.set_completion_source_for_test(
        ["status", "stash"]
            .into_iter()
            .map(|name| {
                sopra_completion::CompletionItem::new(
                    name,
                    "",
                    name,
                    sopra_completion::CompletionKind::Subcommand,
                )
            })
            .collect(),
    );

    state.handle_key(Key::Tab);

    assert_eq!(state.buffer(), "git ");
    assert_eq!(state.selected, Some(0));
    assert_eq!(state.completion_hint(), "Tab to accept");
}

#[test]
fn unselected_completion_prompts_tab_to_focus() {
    let mut state = editor("git ");
    state.set_completion_source_for_test(
        ["status", "stash"]
            .into_iter()
            .map(|name| {
                sopra_completion::CompletionItem::new(
                    name,
                    "",
                    name,
                    sopra_completion::CompletionKind::Subcommand,
                )
            })
            .collect(),
    );

    assert_eq!(state.completion_hint(), "Tab to focus");
}

#[test]
fn single_completion_prompts_tab_to_accept() {
    let mut state = editor("git st");
    state.set_completion_source_for_test(vec![sopra_completion::CompletionItem::new(
        "status",
        "",
        "status",
        sopra_completion::CompletionKind::Subcommand,
    )]);

    assert_eq!(state.completion_hint(), "Tab to accept");
}

#[test]
fn completion_results_start_unselected_so_history_can_handle_navigation() {
    let mut state = editor("git ");
    state.set_completion_source_for_test(vec![sopra_completion::CompletionItem::new(
        "status",
        "",
        "status",
        sopra_completion::CompletionKind::Subcommand,
    )]);

    assert_eq!(state.selected, None);
    assert_eq!(
        state.handle_key(Key::Up).expect("history result").reason,
        ExitReason::DelegateUp
    );
    assert_eq!(state.selected, None);
}

#[test]
fn armed_completion_keeps_the_first_result_selected_after_refresh() {
    let mut state = editor("npm access set mfa=");
    let start = "npm access set mfa=".len();
    state.set_completion_source_for_test(vec![sopra_completion::CompletionItem::with_range(
        "automation",
        "automation",
        None,
        sopra_completion::CompletionKind::Value,
        start..start,
        sopra_completion::CompletionSource::CommandIndex,
    )]);

    assert!(state.arm_first_completion());
    state.reset_selection();

    assert_eq!(state.selected, Some(0));
}

#[test]
fn completion_position_does_not_move_when_selection_changes() {
    let mut state = editor("cd ./s");
    let start = "cd ./s".find("./s").expect("path token");
    let first = sopra_completion::CompletionItem::with_range(
        "src/",
        "src/",
        None,
        sopra_completion::CompletionKind::Directory,
        state.buffer().len()..state.buffer().len(),
        sopra_completion::CompletionSource::Filesystem,
    );
    let second = sopra_completion::CompletionItem::with_range(
        "scripts/",
        "scripts/",
        None,
        sopra_completion::CompletionKind::Directory,
        start..state.buffer().len(),
        sopra_completion::CompletionSource::Filesystem,
    );
    state.set_completion_source_for_test(vec![first, second]);
    let unselected_width = state.completion_token_width();

    assert!(state.arm_first_completion());
    assert!(state.move_selection(1));

    assert_eq!(state.completion_token_width(), unselected_width);
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
