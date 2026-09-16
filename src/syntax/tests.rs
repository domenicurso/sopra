use std::path::Path;

use super::{SyntaxKind, command_position, highlight};

#[test]
fn lexer_marks_real_fake_commands_and_shell_operators() {
    let spans = highlight("echo -n hi | fake", Path::new("."));
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
    let spans = highlight("echo \"hello world\"", Path::new("."));
    assert_eq!(spans.len(), 2);
    assert_eq!(
        &"echo \"hello world\""[spans[1].start..spans[1].end],
        "\"hello world\""
    );
}
