use super::tokens::{DelimiterPair, QuotePair, Substitution, next_char, operator_end};
use super::{SyntaxKind, SyntaxSpan};

#[derive(Default)]
pub(super) struct ScannedWord {
    pub(super) end: usize,
    pub(super) spans: Vec<SyntaxSpan>,
    pub(super) pairs: Vec<QuotePair>,
    pub(super) delimiters: Vec<DelimiterPair>,
    pub(super) substitutions: Vec<Substitution>,
}

pub(super) fn scan_word(line: &str, start: usize) -> ScannedWord {
    let mut scan = WordScanner {
        line,
        word: ScannedWord {
            end: start,
            ..ScannedWord::default()
        },
    };
    while scan.word.end < line.len() {
        let (character, _) = next_char(line, scan.word.end);
        if character.is_whitespace() || operator_end(line, scan.word.end).is_some() {
            break;
        }
        match character {
            '\'' | '"' => scan.quote(character),
            '\\' => scan.escape(),
            '$' => scan.variable(),
            _ if character.is_ascii_digit() => scan.number(),
            _ => scan.argument(),
        }
    }
    scan.word
}

struct WordScanner<'a> {
    line: &'a str,
    word: ScannedWord,
}

impl WordScanner<'_> {
    fn quote(&mut self, quote: char) {
        let open = self.push(self.word.end, self.word.end + 1, SyntaxKind::Quote);
        self.word.end += 1;
        let mut body_start = self.word.end;
        while self.word.end < self.line.len() {
            let (character, next) = next_char(self.line, self.word.end);
            if character == quote {
                self.push(body_start, self.word.end, SyntaxKind::String);
                let close = self.push(self.word.end, next, SyntaxKind::Quote);
                self.word.pairs.push(QuotePair {
                    open,
                    close: Some(close),
                });
                self.word.end = next;
                return;
            }
            if quote == '"' && matches!(character, '\\' | '$') {
                self.push(body_start, self.word.end, SyntaxKind::String);
                if character == '\\' {
                    self.escape();
                } else {
                    self.variable();
                }
                body_start = self.word.end;
            } else {
                self.word.end = next;
            }
        }
        self.push(body_start, self.word.end, SyntaxKind::String);
        self.word.pairs.push(QuotePair { open, close: None });
    }

    fn escape(&mut self) {
        let start = self.word.end;
        self.word.end = next_char(self.line, self.word.end).1;
        if self.word.end < self.line.len() {
            self.word.end = next_char(self.line, self.word.end).1;
        }
        self.push(start, self.word.end, SyntaxKind::Escape);
    }

    fn variable(&mut self) {
        let start = self.word.end;
        if self.line[start..].starts_with("$(") && !self.line[start..].starts_with("$((") {
            let substitution = scan_substitution(self.line, start);
            self.word.end = substitution.end;
            self.word.substitutions.push(substitution);
            return;
        }
        self.word.end += 1;
        if self.line[self.word.end..].starts_with('{') {
            self.parameter(start);
            return;
        }
        while self.word.end < self.line.len() {
            let (character, next) = next_char(self.line, self.word.end);
            if !(character == '_' || character.is_ascii_alphanumeric()) {
                break;
            }
            self.word.end = next;
        }
        self.push(start, self.word.end, SyntaxKind::Variable);
    }

    fn parameter(&mut self, start: usize) {
        self.push(start, start + 1, SyntaxKind::Variable);
        let open = self.push(start + 1, start + 2, SyntaxKind::Punctuation);
        self.word.end += 1;
        let body = self.word.end;
        let close = super::delimiters::parameter_close(self.line, body);
        self.word.end = close.unwrap_or(self.line.len());
        self.push(body, self.word.end, SyntaxKind::Variable);
        let close = close.map(|position| {
            self.word.end = position + 1;
            self.push(position, self.word.end, SyntaxKind::Punctuation)
        });
        self.word.delimiters.push(DelimiterPair { open, close });
    }

    fn number(&mut self) {
        let start = self.word.end;
        while self.word.end < self.line.len() {
            let (character, next) = next_char(self.line, self.word.end);
            if !(character.is_ascii_digit() || character == '.') {
                break;
            }
            self.word.end = next;
        }
        self.push(start, self.word.end, SyntaxKind::Number);
    }

    fn argument(&mut self) {
        let start = self.word.end;
        while self.word.end < self.line.len() {
            let (character, next) = next_char(self.line, self.word.end);
            if character.is_whitespace()
                || operator_end(self.line, self.word.end).is_some()
                || matches!(character, '\'' | '"' | '\\' | '$')
            {
                break;
            }
            self.word.end = next;
        }
        self.push(start, self.word.end, SyntaxKind::Argument);
    }

    fn push(&mut self, start: usize, end: usize, kind: SyntaxKind) -> usize {
        if start < end {
            self.word.spans.push(SyntaxSpan { start, end, kind });
        }
        self.word.spans.len().saturating_sub(1)
    }
}

fn scan_substitution(line: &str, start: usize) -> Substitution {
    let mut cursor = start + 2;
    let mut depth = 1;
    while cursor < line.len() {
        let (character, next) = next_char(line, cursor);
        match character {
            '(' => depth += 1,
            ')' => depth -= 1,
            '#' => {
                cursor = line[cursor..]
                    .find('\n')
                    .map_or(line.len(), |end| cursor + end);
                continue;
            }
            _ if !character.is_whitespace() && operator_end(line, cursor).is_none() => {
                cursor = scan_word(line, cursor).end;
                continue;
            }
            _ => {}
        }
        if depth == 0 {
            return Substitution {
                start,
                close: Some(cursor),
                end: next,
            };
        }
        cursor = next;
    }
    Substitution {
        start,
        close: None,
        end: line.len(),
    }
}
