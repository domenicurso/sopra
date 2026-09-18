use std::path::Path;

use brush_parser::unquote_str;

use super::{
    SyntaxKind, SyntaxSpan, command_available_for_highlight,
    lex::{LexedLine, is_assignment, lex},
};

pub(super) fn highlight(line: &str, cwd: &Path, cursor: Option<usize>) -> Vec<SyntaxSpan> {
    let mut lexed = lex(line);
    classify_words(&mut lexed, cwd);
    mark_quotes(&mut lexed, cursor.map(|value| value.min(line.len())));
    mark_delimiters(&mut lexed, cursor.map(|value| value.min(line.len())));
    lexed.spans
}

fn classify_words(lexed: &mut LexedLine, cwd: &Path) {
    for word in &lexed.words {
        let plain = unquote_str(&word.text);
        let kind = if word.command_expected && !is_assignment(&plain) {
            if command_available_for_highlight(&plain, cwd) {
                SyntaxKind::RealCommand
            } else {
                SyntaxKind::FakeCommand
            }
        } else if !word.command_expected && word.text.starts_with('-') {
            SyntaxKind::Flag
        } else {
            SyntaxKind::Argument
        };
        for span in &mut lexed.spans[word.span_start..word.span_end] {
            if span.kind == SyntaxKind::Argument {
                span.kind = kind;
            }
        }
    }
}

fn mark_quotes(lexed: &mut LexedLine, cursor: Option<usize>) {
    for pair in lexed.quote_pairs.clone() {
        let open = lexed.spans[pair.open];
        if let Some(close_index) = pair.close {
            let close = lexed.spans[close_index];
            if cursor.is_some_and(|cursor| {
                cursor == open.start
                    || cursor == open.end
                    || cursor == close.start
                    || cursor == close.end
            }) {
                lexed.spans[pair.open].kind = SyntaxKind::MatchedQuote;
                lexed.spans[close_index].kind = SyntaxKind::MatchedQuote;
            }
        } else {
            lexed.spans[pair.open].kind = SyntaxKind::Error;
        }
    }
}

fn mark_delimiters(lexed: &mut LexedLine, cursor: Option<usize>) {
    for &index in &lexed.unmatched_delimiters {
        lexed.spans[index].kind = SyntaxKind::Error;
    }
    for pair in lexed.delimiter_pairs.clone() {
        let Some(close_index) = pair.close else {
            lexed.spans[pair.open].kind = SyntaxKind::Error;
            continue;
        };
        let open = lexed.spans[pair.open];
        let close = lexed.spans[close_index];
        if cursor.is_some_and(|cursor| {
            cursor == open.start
                || cursor == open.end
                || cursor == close.start
                || cursor == close.end
        }) {
            lexed.spans[pair.open].kind = SyntaxKind::MatchedQuote;
            lexed.spans[close_index].kind = SyntaxKind::MatchedQuote;
        }
    }
}
