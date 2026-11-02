use super::{
    SyntaxKind, SyntaxSpan,
    delimiters::track,
    scan::scan_word,
    shell_state::ShellState,
    tokens::{DelimiterPair, LexedLine, QuotePair, WordToken, next_char, operator_end},
};

pub(super) fn lex(line: &str) -> LexedLine {
    let mut lexer = Lexer::new(line);
    let mut cursor = 0;
    while cursor < line.len() {
        let (character, next) = next_char(line, cursor);
        if character.is_whitespace() {
            if character == '\n' {
                lexer.result.final_state.consume_operator(";");
            }
            cursor = next;
        } else if let Some(end) = operator_end(line, cursor) {
            lexer.operator(cursor, end);
            cursor = end;
        } else if character == '#' {
            let end = line[cursor..]
                .find('\n')
                .map_or(line.len(), |end| cursor + end);
            lexer.result.spans.push(SyntaxSpan {
                start: cursor,
                end,
                kind: SyntaxKind::Comment,
            });
            cursor = end;
        } else {
            cursor = lexer.word(cursor);
        }
    }
    lexer
        .result
        .unmatched_delimiters
        .extend(lexer.delimiters.into_iter().map(|(_, span)| span));
    lexer.result
}

struct Lexer<'a> {
    line: &'a str,
    result: LexedLine,
    delimiters: Vec<(char, usize)>,
}

impl<'a> Lexer<'a> {
    fn new(line: &'a str) -> Self {
        Self {
            line,
            result: LexedLine {
                spans: Vec::new(),
                words: Vec::new(),
                quote_pairs: Vec::new(),
                delimiter_pairs: Vec::new(),
                unmatched_delimiters: Vec::new(),
                final_state: ShellState::new(),
                substitutions: Vec::new(),
            },
            delimiters: Vec::new(),
        }
    }

    fn operator(&mut self, start: usize, end: usize) {
        let span = self.result.spans.len();
        let operator = &self.line[start..end];
        let kind = if matches!(
            operator,
            "(" | ")" | "[" | "]" | "[[" | "]]" | "{" | "}" | ";" | ";;" | ";&" | ";;&"
        ) {
            SyntaxKind::Punctuation
        } else {
            SyntaxKind::Operator
        };
        self.result.spans.push(SyntaxSpan { start, end, kind });
        if !self.result.final_state.in_case_pattern() || !matches!(operator, "(" | ")") {
            track(operator, span, &mut self.delimiters, &mut self.result);
        }
        self.result.final_state.consume_operator(operator);
    }

    fn word(&mut self, start: usize) -> usize {
        let word = scan_word(self.line, start);
        let span_start = self.result.spans.len();
        self.result.spans.extend(word.spans);
        self.result
            .quote_pairs
            .extend(word.pairs.into_iter().map(|pair| QuotePair {
                open: pair.open + span_start,
                close: pair.close.map(|index| index + span_start),
            }));
        self.result
            .delimiter_pairs
            .extend(word.delimiters.into_iter().map(|pair| DelimiterPair {
                open: pair.open + span_start,
                close: pair.close.map(|index| index + span_start),
            }));
        self.result.substitutions.extend(word.substitutions);
        self.result.words.push(WordToken {
            text: self.line[start..word.end].to_string(),
            span_start,
            span_end: self.result.spans.len(),
            role: self
                .result
                .final_state
                .consume_word(&self.line[start..word.end]),
        });
        word.end
    }
}
