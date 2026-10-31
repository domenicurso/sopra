mod cursor;
mod keys;
mod mouse;
mod resize_signal;
mod terminal;

pub(crate) use self::mouse::{MouseEvent, MouseKind};
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
    Control(char),
    Alt(char),
    Mouse(MouseEvent),
    Ignore,
    Enter,
    Tab,
    BackTab,
    Backspace,
    Delete,
    Escape,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    AltUp,
    AltDown,
}
