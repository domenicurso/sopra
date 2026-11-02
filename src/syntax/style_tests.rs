use std::path::Path;

use ratatui::style::{Color, Modifier};

use super::{highlight_at, highlight_without_cursor, style_at};

#[test]
fn shell_syntax_uses_the_terminal_palette_without_bold_or_dim() {
    let line = "if echo -n \"$HOME\" 42; then /no-such-sopra-command 'text'; fi # comment";
    let spans = highlight_without_cursor(line, Path::new("."));
    for (token, color) in [
        ("if", Color::Yellow),
        ("echo", Color::Green),
        ("-n", Color::Blue),
        ("$HOME", Color::Cyan),
        ("42", Color::Magenta),
        (";", Color::Reset),
        ("/no-such-sopra-command", Color::Red),
        ("text", Color::Magenta),
        ("# comment", Color::DarkGray),
    ] {
        let start = line.find(token).unwrap(); // Each token occurs in the literal fixture.
        let style = style_at(&spans, start);
        assert_eq!(style.fg, Some(color), "{token}");
    }
    assert!(
        spans
            .iter()
            .all(|span| span.style().add_modifier.is_empty())
    );
}

#[test]
fn delimiter_feedback_only_changes_the_active_pair_or_unmatched_delimiter() {
    let line = "(echo hi)";
    let normal = highlight_without_cursor(line, Path::new("."));
    let active = highlight_at(line, Path::new("."), 1);
    for position in [0, 8] {
        assert_eq!(style_at(&normal, position).fg, Some(Color::Reset));
        assert!(style_at(&normal, position).add_modifier.is_empty());
        assert_eq!(style_at(&active, position).fg, Some(Color::Reset));
        assert_eq!(
            style_at(&active, position).add_modifier,
            Modifier::UNDERLINED
        );
    }
    let incomplete = highlight_without_cursor("(echo hi", Path::new("."));
    assert_eq!(style_at(&incomplete, 0).fg, Some(Color::Red));
    assert!(style_at(&incomplete, 0).add_modifier.is_empty());
    let line = "echo \"$(printf hi)\"";
    let active = highlight_at(line, Path::new("."), 8);
    for position in [7, 17] {
        assert_eq!(
            style_at(&active, position).add_modifier,
            Modifier::UNDERLINED
        );
    }
}

#[test]
fn parameter_braces_use_the_same_delimiter_feedback_as_parentheses() {
    let line = "echo ${app##*/}";
    let normal = highlight_without_cursor(line, Path::new("."));
    let active = highlight_at(line, Path::new("."), 7);
    for position in [6, 14] {
        assert_eq!(style_at(&normal, position).fg, Some(Color::Reset));
        assert_eq!(
            style_at(&active, position).add_modifier,
            Modifier::UNDERLINED
        );
    }
    assert_eq!(style_at(&normal, 7).fg, Some(Color::Cyan));
    let incomplete = highlight_without_cursor("echo ${app", Path::new("."));
    assert_eq!(style_at(&incomplete, 6).fg, Some(Color::Red));
}

#[test]
fn conditional_operators_are_colored_separately_from_command_flags_and_delimiters() {
    let line = "if [[ ! -o SHARE_HISTORY && -n \"$HOME\" ]]; then echo -n hi > /dev/null; fi";
    let spans = highlight_without_cursor(line, Path::new("."));
    for (token, color) in [
        ("[[", Color::Reset),
        ("]]", Color::Reset),
        ("!", Color::Yellow),
        ("-o", Color::Yellow),
        ("&&", Color::Yellow),
        ("-n", Color::Yellow),
        (">", Color::Yellow),
        (";", Color::Reset),
    ] {
        let start = line.find(token).unwrap(); // Each token occurs in the literal fixture.
        assert_eq!(style_at(&spans, start).fg, Some(color), "{token}");
    }
    let flag = line.rfind("-n").unwrap(); // The final -n is an ordinary command flag.
    assert_eq!(style_at(&spans, flag).fg, Some(Color::Blue));
    assert!(
        spans
            .iter()
            .all(|span| span.style().add_modifier.is_empty())
    );
}
