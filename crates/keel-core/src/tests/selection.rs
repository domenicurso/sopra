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
