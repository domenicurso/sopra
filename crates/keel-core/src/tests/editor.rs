use crate::{EditorBuffer, HostLine, HostSnapshot, ScreenPoint, TerminalSize};

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
