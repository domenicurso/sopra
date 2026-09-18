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
