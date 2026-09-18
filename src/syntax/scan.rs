use super::lex::{QuotePair, next_char, operator_end};
use super::{SyntaxKind, SyntaxSpan};

pub(super) fn scan_word(line: &str, mut cursor: usize) -> (usize, Vec<SyntaxSpan>, Vec<QuotePair>) {
    let start = cursor;
    let mut spans = Vec::new();
    let mut pairs = Vec::new();
    while cursor < line.len() {
        let (character, _) = next_char(line, cursor);
        if character.is_whitespace()
            || operator_end(line, cursor).is_some()
            || (character == '#' && cursor == start)
        {
            break;
        }
        match character {
            '\'' | '"' => scan_quote(line, &mut cursor, &mut spans, &mut pairs),
            '\\' => scan_escape(line, &mut cursor, &mut spans),
            '$' => scan_variable(line, &mut cursor, &mut spans),
            _ if character.is_ascii_digit() => scan_number(line, &mut cursor, &mut spans),
            _ => scan_argument(line, &mut cursor, &mut spans),
        }
    }
    (cursor, spans, pairs)
}

fn scan_quote(
    line: &str,
    cursor: &mut usize,
    spans: &mut Vec<SyntaxSpan>,
    pairs: &mut Vec<QuotePair>,
) {
    let quote = line[*cursor..].chars().next().unwrap_or('"');
    let open = push_local(
        spans,
        *cursor,
        *cursor + quote.len_utf8(),
        SyntaxKind::Quote,
    );
    *cursor += quote.len_utf8();
    let mut body_start = *cursor;
    while *cursor < line.len() {
        let (character, next) = next_char(line, *cursor);
        if character == quote {
            push_local_if_nonempty(spans, body_start, *cursor, SyntaxKind::String);
            let close = push_local(spans, *cursor, next, SyntaxKind::Quote);
            pairs.push(QuotePair {
                open,
                close: Some(close),
            });
            *cursor = next;
            return;
        }
        if quote == '"' && character == '\\' {
            push_local_if_nonempty(spans, body_start, *cursor, SyntaxKind::String);
            scan_escape(line, cursor, spans);
            body_start = *cursor;
        } else if quote == '"' && character == '$' {
            push_local_if_nonempty(spans, body_start, *cursor, SyntaxKind::String);
            scan_variable(line, cursor, spans);
            body_start = *cursor;
        } else {
            *cursor = next;
        }
    }
    push_local_if_nonempty(spans, body_start, *cursor, SyntaxKind::String);
    pairs.push(QuotePair { open, close: None });
}

fn scan_escape(line: &str, cursor: &mut usize, spans: &mut Vec<SyntaxSpan>) {
    let start = *cursor;
    *cursor = next_char(line, *cursor).1;
    if *cursor < line.len() {
        *cursor = next_char(line, *cursor).1;
    }
    push_local(spans, start, *cursor, SyntaxKind::Escape);
}

fn scan_variable(line: &str, cursor: &mut usize, spans: &mut Vec<SyntaxSpan>) {
    let start = *cursor;
    *cursor += 1;
    if line[*cursor..].starts_with('{') {
        *cursor += 1;
        while *cursor < line.len() {
            *cursor = next_char(line, *cursor).1;
            if line.as_bytes().get(*cursor - 1) == Some(&b'}') {
                break;
            }
        }
    } else {
        while *cursor < line.len() {
            let (character, next) = next_char(line, *cursor);
            if !(character == '_' || character.is_ascii_alphanumeric()) {
                break;
            }
            *cursor = next;
        }
    }
    push_local(spans, start, *cursor, SyntaxKind::Variable);
}

fn scan_number(line: &str, cursor: &mut usize, spans: &mut Vec<SyntaxSpan>) {
    let start = *cursor;
    while *cursor < line.len() {
        let (character, next) = next_char(line, *cursor);
        if !(character.is_ascii_digit() || character == '.') {
            break;
        }
        *cursor = next;
    }
    push_local(spans, start, *cursor, SyntaxKind::Number);
}

fn scan_argument(line: &str, cursor: &mut usize, spans: &mut Vec<SyntaxSpan>) {
    let start = *cursor;
    while *cursor < line.len() {
        let (character, next) = next_char(line, *cursor);
        if character.is_whitespace()
            || operator_end(line, *cursor).is_some()
            || matches!(character, '\'' | '"' | '\\' | '$')
        {
            break;
        }
        *cursor = next;
    }
    if *cursor == start {
        *cursor = next_char(line, *cursor).1;
    }
    push_local(spans, start, *cursor, SyntaxKind::Argument);
}

fn push_local(spans: &mut Vec<SyntaxSpan>, start: usize, end: usize, kind: SyntaxKind) -> usize {
    spans.push(SyntaxSpan { start, end, kind });
    spans.len() - 1
}

fn push_local_if_nonempty(spans: &mut Vec<SyntaxSpan>, start: usize, end: usize, kind: SyntaxKind) {
    if start < end {
        push_local(spans, start, end, kind);
    }
}
