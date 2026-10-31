use super::*;

#[test]
fn whitespace_only_input_keeps_the_completion_menu_empty() {
    let state = editor("   ");
    assert!(state.visible_items().is_empty());
}

#[test]
fn suspended_history_completion_waits_for_editing() {
    let mut state = EditorState::new(EditorConfig {
        buffer: "git st".to_string(),
        cursor_chars: 6,
        arm_completion: false,
        suppress_completion: true,
        completion_enabled: true,
        syntax_highlighting: true,
        cursor_animation: true,
        prompt: "$ ".to_string(),
        rprompt: String::new(),
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

    state.handle_key(Action::Character('x'));

    assert!(!state.completions.suspended());
    assert!(state.overlay_visible());
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
fn open_completion_menu_takes_focus_on_arrow_navigation() {
    let mut state = editor("git ");
    state.set_completion_source_for_test(vec![sopra_completion::CompletionItem::new(
        "status",
        "",
        "status",
        sopra_completion::CompletionKind::Subcommand,
    )]);

    assert_eq!(state.completions.focus.selected(), None);
    assert!(state.handle_key(Action::Up).is_none());
    assert_eq!(state.completions.focus.selected(), Some(0));
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

    assert_eq!(state.completions.focus.selected(), Some(0));
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
fn arrow_navigation_remembers_focus_while_completions_load() {
    let mut state = editor("git ");
    state.handle_key(Action::Up);
    state.set_completion_source_for_test(vec![sopra_completion::CompletionItem::new(
        "status",
        "",
        "status",
        sopra_completion::CompletionKind::Subcommand,
    )]);
    assert_eq!(state.selected_index(), Some(0));
}
