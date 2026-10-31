use super::*;
use sopra_completion::{CompletionItem, CompletionKind};

#[test]
fn normal_tab_never_accepts_a_full_history_command() {
    let mut editor = editor("echo");
    editor.set_completion_source_for_test(vec![CompletionItem::new(
        "echo-other",
        "",
        "echo-other",
        CompletionKind::Command,
    )]);
    editor.handle_key(Action::Tab);
    assert_eq!(editor.buffer(), "echo-other ");
    assert!(
        editor
            .visible_items()
            .iter()
            .all(|item| item.kind != CompletionKind::History)
    );
}

#[test]
fn search_filters_substrings_and_accepts_without_executing() {
    let mut editor = editor("café");
    assert!(editor.handle_key(Action::SearchHistory).is_none());
    assert!(editor.searching_history());
    assert!(editor.autosuggestion_spans().is_empty());
    assert_eq!(editor.visible_items().len(), 1);
    assert!(editor.handle_key(Action::Enter).is_none());
    assert_eq!(editor.buffer(), "echo café 世界");
    assert!(!editor.searching_history());
    assert_eq!(
        editor.handle_key(Action::Enter).unwrap().reason,
        crate::editor::ExitReason::Accepted
    );
}

#[test]
fn cancellation_preserves_the_draft_cursor_and_completion_visibility() {
    for cancel in [Action::Escape, Action::Cancel] {
        let mut editor = editor("unfinished 世界");
        editor.cursor = 4;
        editor.hide_overlay();
        editor.handle_key(Action::SearchHistory);
        editor.handle_key(Action::KillToStart);
        editor.handle_key(Action::Character('é'));
        editor.handle_key(cancel);
        assert_eq!(editor.buffer(), "unfinished 世界");
        assert_eq!(editor.cursor_byte(), 4);
        assert!(!editor.overlay_visible());
        assert!(!editor.searching_history());
    }
}

#[test]
fn search_works_with_completion_and_inline_hints_disabled() {
    let mut editor = editor("");
    editor.history.enabled = false;
    editor.completions.set_suspended(true);
    editor.completions.engine = None;
    editor.handle_key(Action::SearchHistory);
    assert_eq!(editor.visible_items().len(), 2);
    assert_eq!(editor.selected_index(), Some(0));
    editor.handle_key(Action::Down);
    assert_eq!(editor.selected_index(), Some(1));
    editor.handle_key(Action::SearchHistory);
    assert_eq!(editor.selected_index(), Some(0));
    editor.handle_key(Action::Tab);
    assert_eq!(editor.buffer(), "echo café 世界");
    assert!(editor.completions.suspended());
    assert!(editor.visible_items().is_empty());
}

#[test]
fn no_matches_keeps_search_open_and_paste_edits_the_query() {
    let mut editor = editor("missing");
    editor.handle_key(Action::SearchHistory);
    assert!(editor.visible_items().is_empty());
    assert!(editor.handle_key(Action::Enter).is_none());
    assert!(editor.searching_history());
    let frame = Scene::for_editor(
        &editor,
        TerminalSize::new(80, 24),
        std::time::Instant::now(),
    )
    .render();
    let text = frame
        .buffer
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(text.contains("No matches"), "{text}");
    editor.handle_key(Action::KillToStart);
    editor.insert_paste("世界");
    assert_eq!(editor.visible_items().len(), 1);
    editor.handle_key(Action::Backspace);
    assert_eq!(editor.buffer(), "世");
    editor.handle_key(Action::Escape);
    assert_eq!(editor.buffer(), "missing");
}

#[test]
fn search_keeps_multiline_commands_intact() {
    let mut editor = editor("");
    editor.history.entries = vec![Entry {
        command: "echo 'first\nsecond'".into(),
        started: None,
    }];
    editor.handle_key(Action::SearchHistory);
    editor.insert_paste("second");
    editor.handle_key(Action::Enter);
    assert_eq!(editor.buffer(), "echo 'first\nsecond'");
}

#[test]
fn history_breaks_completion_ties_without_creating_or_promoting_invalid_matches() {
    let mut editor = editor("echo ");
    editor.history.entries = vec![Entry {
        command: "echo older arg".into(),
        started: None,
    }];
    let candidate = |name: &str| {
        CompletionItem::with_range(
            name,
            name,
            None,
            CompletionKind::Value,
            5..5,
            sopra_completion::CompletionSource::ShellContext,
        )
    };
    editor.set_completion_source_for_test(vec![candidate("newer"), candidate("older")]);
    assert_eq!(editor.visible_items()[0].insert, "older");
    assert_eq!(editor.visible_items().len(), 2);
    editor.history.entries[0].command = "echo olderish arg".into();
    editor.refresh_suggestions();
    assert_eq!(editor.visible_items()[0].insert, "newer");
}

#[test]
fn recency_preserves_match_quality_and_handles_completed_word_suffixes() {
    let mut editor = editor("echo ");
    let mut strong = CompletionItem::new("newer", "", "newer", CompletionKind::Value);
    strong.score = 20.0;
    let mut recent = CompletionItem::new("older", "", "older", CompletionKind::Value);
    recent.score = 10.0;
    let mut items = vec![strong, recent];
    editor.history.rank_contextual(&mut items, "echo ", 5);
    assert_eq!(items[0].insert, "newer");
    editor.history.entries[0].command = "echo older argument".into();
    items[0].score = 10.0;
    items[1].suffix = " ".into();
    editor.history.rank_contextual(&mut items, "echo ", 5);
    assert_eq!(items[0].insert, "older");
}
