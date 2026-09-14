use crate::{AppState, HostLine, HostSnapshot, Suggestion, SuggestionKind};

#[test]
fn refining_a_selected_query_selects_the_first_new_match() {
    let snapshot = HostSnapshot {
        line: HostLine::new("command e", 9),
        ..HostSnapshot::default()
    };
    let mut app = AppState::from_snapshot(&snapshot);
    app.set_suggestions(vec![
        Suggestion::new("hello", ""),
        Suggestion::new("echo", ""),
        Suggestion::new("env", ""),
    ]);
    app.move_selection(1);

    app.observe(&HostSnapshot {
        line: HostLine::new("command ec", 10),
        redisplay_generation: 2,
        ..snapshot
    });
    app.refresh_suggestions();

    assert_eq!(app.selected_suggestion, Some(0));
    assert_eq!(app.selected().map(|item| item.label.as_str()), Some("echo"));
}

#[test]
fn selection_scrolls_with_one_row_available_in_travel_direction() {
    let snapshot = HostSnapshot {
        line: HostLine::new("command ", 8),
        ..HostSnapshot::default()
    };
    let mut app = AppState::from_snapshot(&snapshot);
    app.set_suggestions(
        (0..20)
            .map(|index| Suggestion::new(format!("item-{index}"), ""))
            .collect(),
    );

    for _ in 0..10 {
        assert!(app.move_selection(1));
    }
    assert_eq!(app.selected_suggestion, Some(9));
    assert_eq!(app.suggestion_viewport_start(), 0);

    assert!(app.move_selection(1));
    assert_eq!(app.selected_suggestion, Some(10));
    assert_eq!(app.suggestion_viewport_start(), 0);

    assert!(app.move_selection(1));
    assert_eq!(app.selected_suggestion, Some(11));
    assert_eq!(app.suggestion_viewport_start(), 1);

    for _ in 0..8 {
        assert!(app.move_selection(1));
    }
    assert_eq!(app.selected_suggestion, Some(19));
    assert_eq!(app.suggestion_viewport_start(), 8);

    for _ in 0..10 {
        assert!(app.move_selection(-1));
    }
    assert_eq!(app.selected_suggestion, Some(9));
    assert_eq!(app.suggestion_viewport_start(), 8);

    assert!(app.move_selection(-1));
    assert_eq!(app.selected_suggestion, Some(8));
    assert_eq!(app.suggestion_viewport_start(), 7);
}

#[test]
fn suggestions_are_fuzzy_filtered_and_keep_match_positions() {
    let snapshot = HostSnapshot {
        line: HostLine::new("grep --matches", 14),
        ..HostSnapshot::default()
    };
    let mut app = AppState::from_snapshot(&snapshot);
    app.set_suggestions(vec![
        Suggestion::new("--files-without-match", "without matches"),
        Suggestion::new("--files-with-matches", "print matching files"),
        Suggestion::new("--ignore-case", "ignore case"),
    ]);

    assert_eq!(app.suggestions.len(), 1);
    assert_eq!(app.suggestions[0].label, "--files-with-matches");
    assert!(!app.suggestions[0].match_indices().is_empty());
}

#[test]
fn path_queries_can_match_the_full_replacement_when_the_label_is_a_segment() {
    let snapshot = HostSnapshot {
        line: HostLine::new("cat /Usr/dom", 11),
        ..HostSnapshot::default()
    };
    let mut app = AppState::from_snapshot(&snapshot);
    app.set_suggestions(vec![
        Suggestion::with_replacement("dom", "", "/Users/dom").with_kind(SuggestionKind::Directory),
    ]);

    assert_eq!(app.suggestions.len(), 1);
    assert_eq!(app.suggestions[0].replacement(), "/Users/dom");
    assert!(!app.suggestions[0].match_indices().is_empty());
}

#[test]
fn path_labels_hide_shared_leading_segments_without_changing_replacements() {
    let snapshot = HostSnapshot {
        line: HostLine::new("cd ../", 6),
        ..HostSnapshot::default()
    };
    let mut app = AppState::from_snapshot(&snapshot);
    app.set_suggestions(vec![
        Suggestion::with_replacement("../aleq/app", "", "../aleq/app")
            .with_kind(SuggestionKind::File),
        Suggestion::with_replacement("../alead/app/", "", "../alead/app/")
            .with_kind(SuggestionKind::Directory),
        Suggestion::with_replacement("../threadline/app/", "", "../threadline/app/")
            .with_kind(SuggestionKind::Directory),
    ]);

    assert_eq!(
        app.suggestions
            .iter()
            .map(|suggestion| suggestion.label.as_str())
            .collect::<Vec<_>>(),
        ["aleq/app", "alead/app/", "threadline/app/"]
    );
    assert_eq!(
        app.suggestions
            .iter()
            .map(Suggestion::replacement)
            .collect::<Vec<_>>(),
        ["../aleq/app", "../alead/app/", "../threadline/app/"]
    );
}

#[test]
fn path_markers_match_their_explicit_prefix_before_compacting_labels() {
    let snapshot = HostSnapshot {
        line: HostLine::new("cd .", 4),
        ..HostSnapshot::default()
    };
    let mut app = AppState::from_snapshot(&snapshot);
    app.set_suggestions(vec![
        Suggestion::with_replacement("./crates/", "", "./crates/")
            .with_kind(SuggestionKind::Directory),
    ]);

    assert_eq!(app.suggestions.len(), 1);
    assert_eq!(app.suggestions[0].label, "crates/");
    assert_eq!(app.suggestions[0].replacement(), "./crates/");
}

#[test]
fn one_path_result_still_uses_a_local_display_segment() {
    let snapshot = HostSnapshot {
        line: HostLine::new("cd ./crates/keel-c", 18),
        ..HostSnapshot::default()
    };
    let mut app = AppState::from_snapshot(&snapshot);
    app.set_suggestions(vec![
        Suggestion::with_replacement("./crates/keel-core", "", "./crates/keel-core")
            .with_kind(SuggestionKind::Directory),
    ]);

    assert_eq!(app.suggestions[0].label, "keel-core");
    assert_eq!(app.suggestions[0].replacement(), "./crates/keel-core");
    assert_eq!(app.completion_token_width(), "keel-c".len() as u16);
}

#[test]
fn path_matching_ignores_completed_parent_segments() {
    let snapshot = HostSnapshot {
        line: HostLine::new("cd ./crates/", 12),
        ..HostSnapshot::default()
    };
    let mut app = AppState::from_snapshot(&snapshot);
    app.set_suggestions(vec![
        Suggestion::with_replacement("./crates/keel-core", "", "./crates/keel-core")
            .with_kind(SuggestionKind::Directory),
        Suggestion::with_replacement("./crates/keel-ui", "", "./crates/keel-ui")
            .with_kind(SuggestionKind::Directory),
    ]);

    assert!(
        app.suggestions
            .iter()
            .all(|suggestion| suggestion.match_indices().is_empty())
    );
}

#[test]
fn fuzzy_parent_segments_drive_matching_and_popup_alignment() {
    let snapshot = HostSnapshot {
        line: HostLine::new("cd ./crates/kec/", 16),
        ..HostSnapshot::default()
    };
    let mut app = AppState::from_snapshot(&snapshot);
    app.set_suggestions(vec![
        Suggestion::with_replacement("./crates/keel-core/src/", "", "./crates/keel-core/src/")
            .with_kind(SuggestionKind::Directory),
        Suggestion::with_replacement(
            "./crates/keel-scheduler/src/",
            "",
            "./crates/keel-scheduler/src/",
        )
        .with_kind(SuggestionKind::Directory),
    ]);

    assert_eq!(
        app.suggestions
            .iter()
            .map(|suggestion| suggestion.label.as_str())
            .collect::<Vec<_>>(),
        ["keel-core/src/", "keel-scheduler/src/"]
    );
    assert!(
        app.suggestions
            .iter()
            .all(|suggestion| !suggestion.match_indices().is_empty())
    );
    assert!(app.suggestions[0].match_indices().contains(&0));
    assert!(app.suggestions[0].match_indices().contains(&5));
    assert!(
        app.suggestions[0]
            .match_indices()
            .iter()
            .all(|&index| index < 9)
    );
    assert_eq!(app.completion_token_width(), 4);
}

#[test]
fn path_queries_match_and_highlight_each_unresolved_segment() {
    let line = "cd ./t/sn";
    let snapshot = HostSnapshot {
        line: HostLine::new(line, line.len()),
        ..HostSnapshot::default()
    };
    let mut app = AppState::from_snapshot(&snapshot);
    app.set_suggestions(vec![
        Suggestion::with_replacement("./tests/snapshots/", "", "./tests/snapshots/")
            .with_kind(SuggestionKind::Directory),
        Suggestion::with_replacement("./.git/description", "", "./.git/description")
            .with_kind(SuggestionKind::File),
        Suggestion::with_replacement(
            "./native/keel_zsh_module_internal.h",
            "",
            "./native/keel_zsh_module_internal.h",
        )
        .with_kind(SuggestionKind::File),
    ]);

    assert_eq!(
        app.suggestions
            .iter()
            .map(|suggestion| suggestion.label.as_str())
            .collect::<Vec<_>>(),
        [
            "tests/snapshots/",
            ".git/description",
            "native/keel_zsh_module_internal.h",
        ]
    );
    assert!(app.suggestions.iter().all(|suggestion| {
        !suggestion.match_indices().is_empty()
            && suggestion
                .match_indices()
                .iter()
                .all(|&index| index < suggestion.label.len())
    }));
    assert_eq!(app.completion_token_width(), 4);
}

#[test]
fn trailing_path_queries_keep_the_junction_under_the_cursor() {
    let line = "cd ./t/sn/";
    let snapshot = HostSnapshot {
        line: HostLine::new(line, line.len()),
        ..HostSnapshot::default()
    };
    let mut app = AppState::from_snapshot(&snapshot);
    app.set_suggestions(vec![
        Suggestion::with_replacement("./tests/snapshots/", "", "./tests/snapshots/")
            .with_kind(SuggestionKind::Directory),
        Suggestion::with_replacement("./native/src/", "", "./native/src/")
            .with_kind(SuggestionKind::Directory),
    ]);

    assert_eq!(app.completion_token_width(), 5);
}

#[test]
fn fuzzy_parent_completion_keeps_each_matching_branch() {
    let line = "cd ../z/d/m";
    let snapshot = HostSnapshot {
        line: HostLine::new(line, line.len()),
        ..HostSnapshot::default()
    };
    let mut app = AppState::from_snapshot(&snapshot);
    app.set_suggestions(vec![
        Suggestion::with_replacement(
            "../snapzy/node_modules/framer-motion/",
            "",
            "../snapzy/node_modules/framer-motion/",
        )
        .with_kind(SuggestionKind::Directory),
        Suggestion::with_replacement("../zenif/docs/modules/", "", "../zenif/docs/modules/")
            .with_kind(SuggestionKind::Directory),
    ]);

    assert_eq!(app.suggestions.len(), 2);
    assert!(app.suggestions.iter().any(|suggestion| {
        suggestion.label == "zenif/docs/modules/"
            && suggestion.replacement() == "../zenif/docs/modules/"
    }));
}

#[test]
fn cached_source_does_not_reappear_for_a_new_completion_context() {
    let first = HostSnapshot {
        line: HostLine::new("command ", 8),
        ..HostSnapshot::default()
    };
    let second = HostSnapshot {
        line: HostLine::new("other ", 6),
        ..HostSnapshot::default()
    };
    let mut app = AppState::from_snapshot(&first);
    app.set_suggestions(vec![Suggestion::new("stale", "")]);
    assert!(app.suggestions_visible());

    app.observe(&second);
    app.refresh_suggestions();
    assert!(!app.suggestions_visible());
}
