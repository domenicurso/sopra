use super::*;

fn output(width: usize, terminal: bool) -> Output {
    Output {
        width,
        terminal,
        color: false,
    }
}

#[test]
fn syntax_report_exposes_byte_ranges_and_escapes_multiline_tokens_when_piped() {
    let source = "if echo 'café\nline' | /no-such-sopra-command; then echo -n hi; fi";
    let report = render(source, Path::new("."), &output(80, false));
    assert!(report.contains("0..2\tKeyword\tYellow\t\"if\""));
    assert!(report.contains("RealCommand\tGreen\t\"echo\""));
    assert!(report.contains("String\tMagenta\t\"café\\nline\""));
    assert!(report.contains("Operator\tYellow\t\"|\""));
    assert!(report.contains("Flag\tBlue\t\"-n\""));
    assert!(report.contains("FakeCommand\tRed"));
    assert!(!report.contains('\x1b'));
}

#[test]
fn terminal_headers_align_with_metadata_and_text_columns() {
    let report = render("echo hello", Path::new("."), &output(80, true));
    let lines: Vec<_> = report.lines().collect();
    for (column, values) in [
        ("Bytes", ["0..4", "5..10"]),
        ("Kind", ["RealCommand", "Argument"]),
        ("Color", ["Green", "Reset"]),
        ("Text", ["\"echo\"", "\"hello\""]),
    ] {
        let start = lines[0].find(column);
        assert_eq!(lines[1].find(values[0]), start, "{report}");
        assert_eq!(lines[2].find(values[1]), start, "{report}");
    }
    assert!(!report.contains('\t'));
}

#[test]
fn narrow_terminals_preserve_long_unicode_tokens_and_spaces_without_overflowing() {
    let source = "echo '界👨‍👩‍👧  a very long string with repeated  spaces'";
    for width in [20, 24, 40, 64, 80] {
        let report = render(source, Path::new("."), &output(width, true));
        assert!(
            report.lines().all(|line| line.width() < width),
            "{width}: {report}"
        );
        assert!(!report.contains('\t'));
        assert!(report.contains("RealCommand"));
    }
    let text = "\"界👨‍👩‍👧  a very long string with repeated  spaces\"";
    let wrapped = wrap_cells(text, 8);
    assert!(wrapped.iter().all(|line| line.width() <= 8));
    assert_eq!(wrapped.concat(), text);
    assert!(wrapped.iter().any(|line| line.contains("👨‍👩‍👧")));
}
