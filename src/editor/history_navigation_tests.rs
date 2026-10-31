use super::*;

#[test]
fn history_cycling_and_menu_navigation_share_the_same_command() {
    let mut editor = editor("echo");
    assert_eq!(editor.selected_index(), Some(0));
    editor.hide_overlay();
    editor.handle_key(Action::Down);
    assert!(!editor.overlay_visible());
    assert_eq!(editor.autosuggestion_spans()[0].text, " older");
    assert_eq!(editor.selected_index(), Some(1));
    editor.handle_key(Action::Up);
    assert_eq!(editor.autosuggestion_spans()[0].text, " café 世界");
    editor.show_overlay();
    editor.handle_key(Action::Down);
    assert_eq!(editor.autosuggestion_spans()[0].text, " older");
    editor.handle_key(Action::Tab);
    assert_eq!(editor.buffer(), "echo older");
}

#[test]
fn history_recall_updates_the_buffer_in_place_and_restores_the_draft() {
    let mut editor = editor("unfinished draft");
    editor.hide_overlay();
    assert!(editor.handle_key(Action::Up).is_none());
    assert_eq!(editor.buffer(), "echo café 世界");
    assert!(editor.handle_key(Action::Up).is_none());
    assert_eq!(editor.buffer(), "echo older");
    editor.handle_key(Action::Down);
    editor.handle_key(Action::Down);
    assert_eq!(editor.buffer(), "unfinished draft");
    assert!(!editor.overlay_visible());
}

#[test]
fn contextual_completion_focus_suppresses_history_and_refresh_preserves_identity() {
    let mut editor = editor("echo");
    editor.set_completion_source_for_test(vec![sopra_completion::CompletionItem::new(
        "echo-other",
        "",
        "echo-other",
        sopra_completion::CompletionKind::Command,
    )]);
    editor.handle_key(Action::Down);
    editor.handle_key(Action::Down);
    assert!(editor.autosuggestion_spans().is_empty());
    editor.refresh_suggestions();
    assert_eq!(editor.selected_index(), Some(2));
    editor.handle_key(Action::Up);
    assert_eq!(editor.autosuggestion_spans()[0].text, " older");
    editor.set_completion_source_for_test(Vec::new());
    assert_eq!(editor.selected_index(), Some(1));
}

#[test]
fn accepting_one_word_preserves_the_chosen_history_match() {
    let mut editor = editor("echo");
    editor.history.entries = ["echo same newer", "echo same older"]
        .into_iter()
        .map(|command| Entry {
            command: command.to_string(),
            started: None,
        })
        .collect();
    editor.refresh_suggestions();
    editor.handle_key(Action::Down);
    editor.handle_key(Action::WordRight);
    assert_eq!(editor.buffer(), "echo same");
    assert_eq!(editor.autosuggestion_spans()[0].text, " older");
    assert_eq!(editor.selected_index(), Some(1));
    editor.handle_key(Action::Tab);
    assert_eq!(editor.buffer(), "echo same older");
}

#[test]
fn inline_cycling_works_when_completion_is_disabled_and_no_menu_is_rendered() {
    let mut editor = editor("echo");
    editor.completions.engine = None;
    editor.completions.set_suspended(true);
    editor.show_overlay();
    editor.refresh_suggestions();
    assert!(editor.visible_items().is_empty());
    editor.handle_key(Action::Down);
    assert_eq!(editor.buffer(), "echo");
    assert_eq!(editor.autosuggestion_spans()[0].text, " older");
}
