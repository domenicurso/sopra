use super::*;

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
    state.handle_key(Action::Tab);
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

    state.handle_key(Action::Tab);
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

    state.handle_key(Action::Tab);

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

    state.handle_key(Action::Tab);

    assert_eq!(state.buffer(), "npm access set mfa=");
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

    state.handle_key(Action::Tab);

    assert_eq!(state.buffer(), "git ");
    assert_eq!(state.completions.focus.selected(), Some(0));
    assert_eq!(state.completion_hint(), "Tab to accept");
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
