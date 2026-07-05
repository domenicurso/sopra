use crate::TerminalSize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputEvent {
    Key(Key),
    Paste(String),
    Resize(TerminalSize),
    FocusGained,
    FocusLost,
    TimerTick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    Tab,
    Enter,
    Esc,
    CtrlC,
}
