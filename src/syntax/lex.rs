use super::{SyntaxKind, SyntaxSpan, delimiters::track, scan::scan_word};

#[derive(Debug, Clone)]
pub(super) struct LexedLine {
    pub(super) spans: Vec<SyntaxSpan>,
    pub(super) words: Vec<WordToken>,
    pub(super) quote_pairs: Vec<QuotePair>,
    pub(super) delimiter_pairs: Vec<DelimiterPair>,
    pub(super) unmatched_delimiters: Vec<usize>,
    pub(super) final_state: ShellState,
}

#[derive(Debug, Clone)]
pub(super) struct WordToken {
    pub(super) text: String,
    pub(super) span_start: usize,
    pub(super) span_end: usize,
    pub(super) command_expected: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct QuotePair {
    pub(super) open: usize,
    pub(super) close: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct DelimiterPair {
    pub(super) open: usize,
    pub(super) close: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ShellState {
    pub(super) command_expected: bool,
    redirect_expected: bool,
}

impl ShellState {
    const fn new() -> Self {
        Self {
            command_expected: true,
            redirect_expected: false,
        }
    }
}

pub(super) fn lex(line: &str) -> LexedLine {
    let mut result = LexedLine {
        spans: Vec::new(),
        words: Vec::new(),
        quote_pairs: Vec::new(),
        delimiter_pairs: Vec::new(),
        unmatched_delimiters: Vec::new(),
        final_state: ShellState::new(),
    };
    let mut state = result.final_state;
    let mut delimiters = Vec::new();
    let mut cursor = 0;
    while cursor < line.len() {
        let (character, next) = next_char(line, cursor);
        if character.is_whitespace() {
            cursor = next;
            continue;
        }
        if let Some(end) = operator_end(line, cursor) {
            let span = result.spans.len();
            result.spans.push(SyntaxSpan {
                start: cursor,
                end,
                kind: SyntaxKind::Operator,
            });
            track(&line[cursor..end], span, &mut delimiters, &mut result);
            consume_operator(&line[cursor..end], &mut state);
            cursor = end;
            continue;
        }
        if character == '#' {
            result.spans.push(SyntaxSpan {
                start: cursor,
                end: line.len(),
                kind: SyntaxKind::Comment,
            });
            break;
        }
        let word_start = cursor;
        let span_start = result.spans.len();
        let (end, spans, pairs) = scan_word(line, cursor);
        let offset = result.spans.len();
        result.spans.extend(spans);
        result
            .quote_pairs
            .extend(pairs.into_iter().map(|pair| QuotePair {
                open: pair.open + offset,
                close: pair.close.map(|index| index + offset),
            }));
        result.words.push(WordToken {
            text: line[word_start..end].to_string(),
            span_start,
            span_end: result.spans.len(),
            command_expected: state.command_expected,
        });
        consume_word(&line[word_start..end], &mut state);
        cursor = end;
    }
    result
        .unmatched_delimiters
        .extend(delimiters.into_iter().map(|(_, span)| span));
    result.final_state = state;
    result
}

pub(super) fn next_char(line: &str, cursor: usize) -> (char, usize) {
    line[cursor..]
        .chars()
        .next()
        .map_or(('\0', cursor), |character| {
            (character, cursor + character.len_utf8())
        })
}

pub(super) fn operator_end(line: &str, cursor: usize) -> Option<usize> {
    [
        ";;&", ">>", "<<", "&&", "||", ";;", ";&", "&>", ">|", "|", "&", ";", "(", ")", "[", "]",
        "{", "}", "<", ">",
    ]
    .into_iter()
    .find_map(|operator| {
        line[cursor..]
            .starts_with(operator)
            .then(|| cursor + operator.len())
    })
}

fn consume_word(word: &str, state: &mut ShellState) {
    if state.redirect_expected {
        state.redirect_expected = false;
    } else if state.command_expected && !is_assignment(word) {
        state.command_expected = false;
    }
}

fn consume_operator(operator: &str, state: &mut ShellState) {
    state.redirect_expected = operator
        .chars()
        .any(|character| matches!(character, '<' | '>'));
    if matches!(
        operator,
        "|" | "||" | "&&" | ";" | ";&" | ";;" | ";;&" | "&"
    ) {
        state.command_expected = true;
    } else if !state.redirect_expected {
        state.command_expected = false;
    }
}

pub(super) fn is_assignment(word: &str) -> bool {
    let Some((name, _)) = word.split_once('=') else {
        return false;
    };
    !name.is_empty()
        && name.chars().enumerate().all(|(index, character)| {
            character == '_'
                || character.is_ascii_alphabetic() && index == 0
                || character.is_ascii_digit() && index > 0
        })
}
