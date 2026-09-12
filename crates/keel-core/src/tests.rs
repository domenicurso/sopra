use super::*;

#[test]
fn edits_are_grapheme_aware() {
    let mut buffer = EditorBuffer::new("a🙂é", 3);
    assert_eq!(buffer.grapheme_count(), 3);
    assert_eq!(buffer.cursor_byte_offset(), buffer.text().len());
    buffer.backspace();
    assert_eq!(buffer.text(), "a🙂");
    assert_eq!(buffer.cursor(), 2);
    buffer.move_left();
    buffer.delete();
    assert_eq!(buffer.text(), "a");
    buffer.insert("界");
    assert_eq!(buffer.text(), "a界");
    assert_eq!(buffer.cursor_width(), 3);
}

#[test]
fn host_cursor_is_clamped_to_graphemes_and_terminal() {
    let snapshot = HostSnapshot {
        line: HostLine::new("🙂", 99),
        terminal: TerminalSize::new(10, 4),
        cursor: ScreenPoint {
            column: 99,
            row: 99,
        },
        ..HostSnapshot::default()
    }
    .sanitized();
    assert_eq!(snapshot.line.cursor, 1);
    assert_eq!(snapshot.cursor, ScreenPoint { column: 9, row: 3 });
}

#[test]
fn codepoint_cursor_is_translated_to_a_grapheme_cursor() {
    let line = HostLine::from_codepoint_cursor("a é", 3);
    assert_eq!(line.text, "a é");
    assert_eq!(line.cursor, 3);
}

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
