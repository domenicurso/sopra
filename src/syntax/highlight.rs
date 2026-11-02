use std::path::Path;

use brush_parser::unquote_str;

use super::{
    SyntaxKind, SyntaxSpan, command_available_for_highlight,
    lex::lex,
    shell_state::WordRole,
    tokens::{LexedLine, Substitution},
};

pub(super) fn highlight(line: &str, cwd: &Path, cursor: Option<usize>) -> Vec<SyntaxSpan> {
    let mut lexed = lex(line);
    classify_words(&mut lexed, cwd);
    mark_quotes(&mut lexed, cursor.map(|value| value.min(line.len())));
    mark_delimiters(&mut lexed, cursor.map(|value| value.min(line.len())));
    for substitution in &lexed.substitutions {
        lexed
            .spans
            .extend(highlight_substitution(substitution, line, cwd, cursor));
    }
    lexed.spans.sort_by_key(|span| span.start);
    lexed.spans
}

fn classify_words(lexed: &mut LexedLine, cwd: &Path) {
    for word in &lexed.words {
        let plain = unquote_str(&word.text);
        let kind = if word.role == WordRole::Keyword {
            SyntaxKind::Keyword
        } else if word.role == WordRole::Operator {
            SyntaxKind::Operator
        } else if word.role == WordRole::Command {
            if command_available_for_highlight(&plain, cwd) {
                SyntaxKind::RealCommand
            } else {
                SyntaxKind::FakeCommand
            }
        } else if word.text.starts_with('-') {
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

fn highlight_substitution(
    substitution: &Substitution,
    line: &str,
    cwd: &Path,
    cursor: Option<usize>,
) -> Vec<SyntaxSpan> {
    let body_start = substitution.start + 2;
    let body_end = substitution.close.unwrap_or(substitution.end);
    let kind = substitution_delimiter_kind(substitution, cursor);
    let mut spans = vec![
        SyntaxSpan {
            start: substitution.start,
            end: substitution.start + 1,
            kind: SyntaxKind::Variable,
        },
        SyntaxSpan {
            start: substitution.start + 1,
            end: body_start,
            kind,
        },
    ];
    let inner_cursor = cursor
        .filter(|&cursor| cursor >= body_start && cursor <= body_end)
        .map(|cursor| cursor - body_start);
    spans.extend(
        highlight(&line[body_start..body_end], cwd, inner_cursor)
            .into_iter()
            .map(|span| SyntaxSpan {
                start: span.start + body_start,
                end: span.end + body_start,
                kind: span.kind,
            }),
    );
    if let Some(close) = substitution.close {
        spans.push(SyntaxSpan {
            start: close,
            end: substitution.end,
            kind,
        });
    }
    spans
}

fn substitution_delimiter_kind(substitution: &Substitution, cursor: Option<usize>) -> SyntaxKind {
    let touching = cursor.is_some_and(|cursor| {
        cursor == substitution.start + 1
            || cursor == substitution.start + 2
            || substitution
                .close
                .is_some_and(|close| cursor == close || cursor == close + 1)
    });
    if substitution.close.is_none() {
        SyntaxKind::Error
    } else if touching {
        SyntaxKind::MatchedQuote
    } else {
        SyntaxKind::Punctuation
    }
}
