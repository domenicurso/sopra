use std::path::Path;

use super::{SyntaxKind, command_position, highlight_at};

#[test]
fn lexer_marks_real_fake_commands_and_shell_operators() {
    let line = "echo -n hi | fake";
    let spans = highlight_at(line, Path::new("."), line.len());
    assert_eq!(spans[0].kind, SyntaxKind::RealCommand);
    assert_eq!(spans[1].kind, SyntaxKind::Flag);
    assert_eq!(spans[2].kind, SyntaxKind::Argument);
    assert_eq!(spans[3].kind, SyntaxKind::Operator);
    assert_eq!(spans[4].kind, SyntaxKind::FakeCommand);
}

#[test]
fn command_position_follows_shell_command_boundaries() {
    assert!(command_position("ec", 2));
    assert!(!command_position("echo ", 5));
    assert!(command_position("echo | ec", 9));
}

#[test]
fn quoted_words_keep_their_lexed_span() {
    let line = "echo \"hello world\"";
    let spans = highlight_at(line, Path::new("."), line.len());
    assert!(spans.iter().any(|span| span.kind == SyntaxKind::String));
    assert!(
        spans
            .iter()
            .any(|span| { matches!(span.kind, SyntaxKind::Quote | SyntaxKind::MatchedQuote) })
    );
}

#[test]
fn incomplete_shell_words_still_receive_lexical_styles() {
    let line = "echo 42 \"$HOME";
    let spans = highlight_at(line, Path::new("."), line.len());
    assert!(spans.iter().any(|span| span.kind == SyntaxKind::Variable));
    assert!(spans.iter().any(|span| span.kind == SyntaxKind::Number));
    assert!(spans.iter().any(|span| span.kind == SyntaxKind::Error));
}

#[test]
fn an_unclosed_quote_keeps_the_command_style_and_marks_the_quote_as_error() {
    let line = "echo \"";
    let spans = highlight_at(line, Path::new("."), line.len());
    assert_eq!(spans[0].kind, SyntaxKind::RealCommand);
    assert!(spans.iter().any(|span| span.kind == SyntaxKind::Error));
}

#[test]
fn a_quote_next_to_the_cursor_marks_its_pair() {
    let line = "echo \"hi\"";
    let spans = super::highlight_at(line, Path::new("."), 6);
    assert_eq!(
        spans
            .iter()
            .filter(|span| span.kind == SyntaxKind::MatchedQuote)
            .count(),
        2
    );
}

#[test]
fn delimiters_mark_matching_pairs_and_unmatched_closers() {
    let line = "echo ([value])";
    let spans = highlight_at(line, Path::new("."), 6);
    assert_eq!(
        spans
            .iter()
            .filter(|span| span.kind == SyntaxKind::MatchedQuote)
            .count(),
        4
    );
    let unmatched = highlight_at("echo ([value]", Path::new("."), 13);
    assert!(unmatched.iter().any(|span| span.kind == SyntaxKind::Error));
    let unmatched_close = highlight_at("echo value]", Path::new("."), 11);
    assert!(
        unmatched_close
            .iter()
            .any(|span| span.kind == SyntaxKind::Error)
    );
}

#[test]
fn compound_commands_keep_keywords_assignments_and_commands_distinct() {
    let line =
        "while IFS= read -r app; do\n id=$(/bin/sh -c 'echo hi')\n printf '%s' \"$id\"\ndone";
    let spans = super::highlight_without_cursor(line, Path::new("."));
    for (token, kind) in [
        ("while", SyntaxKind::Keyword),
        ("IFS=", SyntaxKind::Argument),
        ("read", SyntaxKind::RealCommand),
        ("do", SyntaxKind::Keyword),
        ("id=", SyntaxKind::Argument),
        ("/bin/sh", SyntaxKind::RealCommand),
        ("printf", SyntaxKind::RealCommand),
        ("done", SyntaxKind::Keyword),
    ] {
        let start = line.find(token).unwrap(); // Every token is present in the literal fixture.
        assert!(
            spans
                .iter()
                .any(|span| span.start == start && span.kind == kind),
            "{token}: {spans:?}"
        );
    }
}

#[test]
fn nested_substitutions_work_inside_quotes_without_reclassifying_outer_arguments() {
    let line = "echo \"$(printf '%s' \"$(echo hi)\")\" done '$(fake)'";
    let spans = super::highlight_without_cursor(line, Path::new("."));
    let commands: Vec<_> = spans
        .iter()
        .filter(|span| span.kind == SyntaxKind::RealCommand)
        .map(|span| &line[span.start..span.end])
        .collect();
    assert_eq!(commands, ["echo", "printf", "echo"]);
    assert!(!spans.iter().any(|span| matches!(
        span.kind,
        SyntaxKind::FakeCommand | SyntaxKind::Keyword | SyntaxKind::Error
    )));
    assert!(spans.windows(2).all(|pair| pair[0].end <= pair[1].start));
}

#[test]
fn substitutions_preserve_quoted_parentheses_and_mark_incomplete_pairs() {
    for line in ["echo $(printf ')')", "echo $(echo hi # )\n)"] {
        let spans = super::highlight_without_cursor(line, Path::new("."));
        assert!(
            !spans.iter().any(|span| span.kind == SyntaxKind::Error),
            "{line}"
        );
    }
    let line = "echo $(fake";
    let spans = highlight_at(line, Path::new("."), line.len());
    assert!(
        spans
            .iter()
            .any(|span| span.start == 6 && span.kind == SyntaxKind::Error)
    );
    assert!(
        spans
            .iter()
            .any(|span| span.start == 7 && span.kind == SyntaxKind::FakeCommand)
    );
}

#[test]
fn newlines_end_comments_and_commands_but_escaped_newlines_do_not() {
    let line = "echo hi # comment\nprintf \\\n --help";
    let spans = super::highlight_without_cursor(line, Path::new("."));
    assert!(spans.iter().any(
        |span| &line[span.start..span.end] == "printf" && span.kind == SyntaxKind::RealCommand
    ));
    assert!(
        spans
            .iter()
            .any(|span| &line[span.start..span.end] == "--help" && span.kind == SyntaxKind::Flag)
    );
}

#[test]
fn loop_headers_and_case_patterns_do_not_become_unknown_commands_or_unmatched_pairs() {
    let line = "for app in one two; do case $app in (one|two) echo hi;; *) printf bye;; esac; done";
    let spans = super::highlight_without_cursor(line, Path::new("."));
    assert!(
        !spans
            .iter()
            .any(|span| matches!(span.kind, SyntaxKind::FakeCommand | SyntaxKind::Error))
    );
    let keywords: Vec<_> = spans
        .iter()
        .filter(|span| span.kind == SyntaxKind::Keyword)
        .map(|span| &line[span.start..span.end])
        .collect();
    assert_eq!(keywords, ["for", "in", "do", "case", "in", "esac", "done"]);
}
