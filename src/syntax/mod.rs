mod commands;

use std::path::Path;

use brush_parser::{Token, tokenize_str};
use ratatui::style::{Color, Modifier, Style};

use commands::command_available;

pub(crate) use commands::catalog;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SyntaxKind {
    RealCommand,
    FakeCommand,
    Flag,
    Operator,
    Argument,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SyntaxSpan {
    start: usize,
    end: usize,
    kind: SyntaxKind,
}

impl SyntaxSpan {
    pub(crate) fn style(self) -> Style {
        match self.kind {
            SyntaxKind::RealCommand => Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            SyntaxKind::FakeCommand => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            SyntaxKind::Flag => Style::default().fg(Color::Yellow),
            SyntaxKind::Operator => Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            SyntaxKind::Argument => Style::default().fg(Color::Reset),
        }
    }
}

pub(crate) fn highlight(line: &str, cwd: &Path) -> Vec<SyntaxSpan> {
    let Ok(tokens) = tokenize_str(line) else {
        return Vec::new();
    };
    let mut state = ShellState::new();
    tokens
        .into_iter()
        .map(|token| {
            let location = token.location();
            let start = char_to_byte(line, location.start.index);
            let end = char_to_byte(line, location.end.index);
            let kind = match &token {
                Token::Operator(operator, _) => {
                    consume_operator(operator, &mut state);
                    SyntaxKind::Operator
                }
                Token::Word(word, _) => {
                    let kind = word_kind(word, &state, cwd);
                    consume_word(word, &mut state);
                    kind
                }
            };
            SyntaxSpan { start, end, kind }
        })
        .collect()
}

pub(crate) fn style_at(spans: &[SyntaxSpan], byte: usize) -> Style {
    spans
        .iter()
        .find(|span| span.start <= byte && byte < span.end)
        .copied()
        .map_or_else(|| Style::default().fg(Color::Reset), SyntaxSpan::style)
}

pub(crate) fn command_position(line: &str, cursor: usize) -> bool {
    let prefix = &line[..cursor.min(line.len())];
    let Ok(tokens) = tokenize_str(prefix) else {
        return false;
    };
    let cursor_chars = prefix.chars().count();
    let mut state = ShellState::new();
    for token in tokens {
        let location = token.location();
        if matches!(&token, Token::Word(..))
            && location.start.index <= cursor_chars
            && cursor_chars <= location.end.index
        {
            return state.command_expected && !is_assignment(token.to_str());
        }
        match &token {
            Token::Operator(operator, _) => consume_operator(operator, &mut state),
            Token::Word(word, _) => consume_word(word, &mut state),
        }
    }
    state.command_expected
}

struct ShellState {
    command_expected: bool,
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

fn word_kind(word: &str, state: &ShellState, cwd: &Path) -> SyntaxKind {
    if state.redirect_expected {
        return SyntaxKind::Argument;
    }
    if state.command_expected && !is_assignment(word) {
        return if command_available(word, cwd) {
            SyntaxKind::RealCommand
        } else {
            SyntaxKind::FakeCommand
        };
    }
    if !state.command_expected && word.starts_with('-') {
        SyntaxKind::Flag
    } else {
        SyntaxKind::Argument
    }
}

fn consume_word(word: &str, state: &mut ShellState) {
    if state.redirect_expected {
        state.redirect_expected = false;
    } else if state.command_expected && !is_assignment(word) {
        state.command_expected = false;
    }
}

fn consume_operator(operator: &str, state: &mut ShellState) {
    state.redirect_expected = is_redirection(operator);
    if is_command_boundary(operator) {
        state.command_expected = true;
    } else if !state.redirect_expected {
        state.command_expected = false;
    }
}

fn is_command_boundary(operator: &str) -> bool {
    matches!(
        operator,
        "|" | "||" | "&&" | ";" | ";&" | ";;" | ";;&" | "&" | "\n"
    )
}

fn is_redirection(operator: &str) -> bool {
    operator
        .chars()
        .any(|character| matches!(character, '<' | '>'))
}

fn is_assignment(word: &str) -> bool {
    let Some((name, _)) = word.split_once('=') else {
        return false;
    };
    !name.is_empty()
        && name.chars().enumerate().all(|(index, character)| {
            character == '_'
                || character.is_ascii_alphanumeric() && index > 0
                || character.is_ascii_alphabetic()
        })
}

fn char_to_byte(text: &str, index: usize) -> usize {
    text.char_indices()
        .nth(index)
        .map_or(text.len(), |(byte, _)| byte)
}

#[cfg(test)]
mod tests;
