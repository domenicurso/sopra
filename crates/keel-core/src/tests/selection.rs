use crate::{AppState, HostLine, HostSnapshot, Suggestion};

#[test]
fn selection_wraps_without_owning_the_host_line() {
    let snapshot = HostSnapshot {
        line: HostLine::new("git", 3),
        ..HostSnapshot::default()
    };
    let mut app = AppState::from_snapshot(&snapshot);
    app.set_suggestions(vec![
        Suggestion::new("run git", ""),
        Suggestion::new("inspect git", ""),
        Suggestion::new("search git", ""),
    ]);
    assert!(app.move_selection(-1));
    assert_eq!(
        app.selected().map(|suggestion| suggestion.label.as_str()),
        Some("search git")
    );
    assert_eq!(app.buffer.text(), "git");
}

#[test]
fn dismissed_overlay_stays_hidden_until_the_line_changes() {
    let snapshot = HostSnapshot {
        line: HostLine::new("git", 3),
        ..HostSnapshot::default()
    };
    let mut app = AppState::from_snapshot(&snapshot);
    app.set_suggestions(vec![Suggestion::new("run git", "")]);
    assert!(app.suggestions_visible());
    assert!(app.dismiss_overlay());
    assert!(!app.suggestions_visible());

    let same_line = HostSnapshot {
        redisplay_generation: 2,
        ..snapshot.clone()
    };
    app.observe(&same_line);
    assert!(!app.suggestions_visible());

    let changed_line = HostSnapshot {
        line: HostLine::new("git ", 4),
        ..same_line
    };
    app.observe(&changed_line);
    app.set_suggestions(vec![Suggestion::new("run git", "")]);
    assert!(app.suggestions_visible());
}

#[test]
fn moving_selection_does_not_reopen_a_dismissed_overlay() {
    let snapshot = HostSnapshot {
        line: HostLine::new("git", 3),
        ..HostSnapshot::default()
    };
    let mut app = AppState::from_snapshot(&snapshot);
    app.set_suggestions(vec![
        Suggestion::new("run git", ""),
        Suggestion::new("inspect git", ""),
    ]);
    app.dismiss_overlay();
    assert!(!app.move_selection(1));
    assert!(!app.suggestions_visible());
}

#[test]
fn selection_is_empty_until_down_moves_into_the_list() {
    let snapshot = HostSnapshot {
        line: HostLine::new("git", 3),
        ..HostSnapshot::default()
    };
    let mut app = AppState::from_snapshot(&snapshot);
    app.set_suggestions(vec![
        Suggestion::new("git", ""),
        Suggestion::new("git-receive-pack", ""),
    ]);

    assert_eq!(app.selected_suggestion, None);
    assert!(app.move_selection(1));
    assert_eq!(app.selected_suggestion, Some(0));
}

#[test]
fn a_single_unselected_suggestion_can_be_accepted() {
    let snapshot = HostSnapshot {
        line: HostLine::new("gre", 3),
        ..HostSnapshot::default()
    };
    let mut app = AppState::from_snapshot(&snapshot);
    app.set_suggestions(vec![Suggestion::new("grep", "command")]);

    assert_eq!(app.selected_suggestion, None);
    assert_eq!(app.selected_replacement(), Some("grep"));
}

#[test]
fn an_empty_line_never_displays_cached_suggestions() {
    let mut app = AppState::default();
    app.set_suggestions(vec![Suggestion::new("grep", "command")]);

    assert!(!app.suggestions_visible());
}

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
