mod context;
mod delimiters;
mod highlight;
mod lex;
mod scan;
mod shell_state;
#[cfg(test)]
mod style_tests;
#[cfg(test)]
mod tests;
mod tokens;

use std::path::Path;

use ratatui::style::{Color, Modifier, Style};

use sopra_shell_context::command_available;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SyntaxKind {
    Keyword,
    RealCommand,
    FakeCommand,
    Flag,
    Operator,
    Punctuation,
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
        let color = match self.kind {
            SyntaxKind::Keyword | SyntaxKind::Operator => Color::Yellow,
            SyntaxKind::Flag => Color::Blue,
            SyntaxKind::RealCommand => Color::Green,
            SyntaxKind::String | SyntaxKind::Quote | SyntaxKind::Number => Color::Magenta,
            SyntaxKind::FakeCommand | SyntaxKind::Error => Color::Red,
            SyntaxKind::Variable => Color::Cyan,
            SyntaxKind::Comment => Color::DarkGray,
            SyntaxKind::Punctuation
            | SyntaxKind::Argument
            | SyntaxKind::MatchedQuote
            | SyntaxKind::Escape => Color::Reset,
        };
        let style = Style::default().fg(color);
        if self.kind == SyntaxKind::MatchedQuote {
            style.add_modifier(Modifier::UNDERLINED)
        } else {
            style
        }
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

#[cfg(test)]
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
