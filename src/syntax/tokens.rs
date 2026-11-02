use super::{
    SyntaxSpan,
    shell_state::{ShellState, WordRole},
};

#[derive(Debug, Clone)]
pub(super) struct LexedLine {
    pub(super) spans: Vec<SyntaxSpan>,
    pub(super) words: Vec<WordToken>,
    pub(super) quote_pairs: Vec<QuotePair>,
    pub(super) delimiter_pairs: Vec<DelimiterPair>,
    pub(super) unmatched_delimiters: Vec<usize>,
    pub(super) final_state: ShellState,
    pub(super) substitutions: Vec<Substitution>,
}

#[derive(Debug, Clone)]
pub(super) struct WordToken {
    pub(super) text: String,
    pub(super) span_start: usize,
    pub(super) span_end: usize,
    pub(super) role: WordRole,
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

#[derive(Debug, Clone)]
pub(super) struct Substitution {
    pub(super) start: usize,
    pub(super) close: Option<usize>,
    pub(super) end: usize,
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
        "[[", "]]", ";;&", ">>", "<<", "&&", "||", ";;", ";&", "&>", ">|", "|", "&", ";", "(", ")",
        "[", "]", "{", "}", "<", ">",
    ]
    .into_iter()
    .find_map(|operator| {
        line[cursor..]
            .starts_with(operator)
            .then(|| cursor + operator.len())
    })
}
