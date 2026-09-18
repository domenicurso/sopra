mod commands;
mod context;
mod delimiters;
mod highlight;
mod lex;
mod scan;
#[cfg(test)]
mod tests;

use std::path::Path;

use ratatui::style::{Color, Modifier, Style};

pub(crate) use commands::catalog;
use commands::command_available;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SyntaxKind {
    RealCommand,
    FakeCommand,
    Flag,
    Operator,
    Argument,
    String,
    Number,
    Variable,
    Comment,
    Quote,
    MatchedQuote,
    Error,
    Escape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SyntaxSpan {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) kind: SyntaxKind,
}

impl SyntaxSpan {
    pub(crate) fn style(self) -> Style {
        let (color, modifier) = match self.kind {
            SyntaxKind::RealCommand => (Color::Green, Modifier::BOLD),
            SyntaxKind::FakeCommand => (Color::Red, Modifier::BOLD),
            SyntaxKind::Flag => (Color::Yellow, Modifier::empty()),
            SyntaxKind::Operator => (Color::Cyan, Modifier::BOLD),
            SyntaxKind::Argument => (Color::Reset, Modifier::empty()),
            SyntaxKind::String => (Color::LightMagenta, Modifier::empty()),
            SyntaxKind::Number => (Color::LightBlue, Modifier::empty()),
            SyntaxKind::Variable => (Color::LightCyan, Modifier::empty()),
            SyntaxKind::Comment => (Color::DarkGray, Modifier::DIM),
            SyntaxKind::Quote => (Color::LightMagenta, Modifier::empty()),
            SyntaxKind::MatchedQuote => (Color::LightYellow, Modifier::UNDERLINED),
            SyntaxKind::Error => (Color::LightRed, Modifier::UNDERLINED),
            SyntaxKind::Escape => (Color::Cyan, Modifier::empty()),
        };
        Style::default().fg(color).add_modifier(modifier)
    }
}

pub(crate) fn highlight_at(line: &str, cwd: &Path, cursor: usize) -> Vec<SyntaxSpan> {
    highlight::highlight(line, cwd, Some(cursor))
}

pub(crate) fn highlight_without_cursor(line: &str, cwd: &Path) -> Vec<SyntaxSpan> {
    highlight::highlight(line, cwd, None)
}

pub(crate) fn style_at(spans: &[SyntaxSpan], byte: usize) -> Style {
    spans
        .iter()
        .find(|span| span.start <= byte && byte < span.end)
        .copied()
        .map_or_else(|| Style::default().fg(Color::Reset), SyntaxSpan::style)
}

pub(crate) fn command_position(line: &str, cursor: usize) -> bool {
    context::command_position(line, cursor)
}

pub(crate) fn quote_context(line: &str, cursor: usize) -> Option<char> {
    context::quote_context(line, cursor)
}

pub(crate) fn comment_context(line: &str, cursor: usize) -> bool {
    context::comment_context(line, cursor)
}

pub(crate) fn command_available_for_highlight(word: &str, cwd: &Path) -> bool {
    command_available(word, cwd)
}
