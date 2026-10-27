mod cursor;
mod keys;
mod resize_signal;
mod terminal;

pub(crate) use self::terminal::Terminal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TerminalSize {
    pub(crate) columns: u16,
    pub(crate) rows: u16,
}

impl TerminalSize {
    pub(crate) fn new(columns: u16, rows: u16) -> Self {
        Self {
            columns: columns.max(1),
            rows: rows.max(1),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CursorPosition {
    pub(crate) column: u16,
    pub(crate) row: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Key {
    Character(char),
    Enter,
    Tab,
    Backspace,
    Delete,
    WordBackspace,
    KillToEnd,
    KillToStart,
    Yank,
    Escape,
    Left,
    Right,
    WordLeft,
    WordRight,
    Up,
    Down,
    Home,
    End,
    Cancel,
    Clear,
    PromptWidgets,
    Eof,
}
