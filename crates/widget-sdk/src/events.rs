use std::io;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Activate,
    Key(Key),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Character(char),
    Enter,
    Tab,
    BackTab,
    Escape,
    Cancel,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    Backspace,
    Delete,
    WordLeft,
    WordRight,
    WordBackspace,
    KillToStart,
    KillToEnd,
    Yank,
    PreviousSuggestion,
    NextSuggestion,
    Clear,
    Eof,
}

impl Event {
    pub(super) fn encode(self) -> String {
        use Key::*;
        match self {
            Self::Activate => "activate".into(),
            Self::Key(Character(ch)) => format!("char:{:x}", ch as u32),
            Self::Key(key) => match key {
                Enter => "enter",
                Tab => "tab",
                BackTab => "back-tab",
                Escape => "escape",
                Cancel => "cancel",
                Left => "left",
                Right => "right",
                Up => "up",
                Down => "down",
                Home => "home",
                End => "end",
                Backspace => "backspace",
                Delete => "delete",
                WordLeft => "word-left",
                WordRight => "word-right",
                WordBackspace => "word-backspace",
                KillToStart => "kill-start",
                KillToEnd => "kill-end",
                Yank => "yank",
                PreviousSuggestion => "previous",
                NextSuggestion => "next",
                Clear => "clear",
                Eof => "eof",
                Character(_) => unreachable!("characters are encoded before named keys"),
            }
            .into(),
        }
    }

    pub(super) fn decode(text: &str) -> io::Result<Self> {
        use Key::*;
        if text == "activate" {
            return Ok(Self::Activate);
        }
        if let Some(value) = text.strip_prefix("char:") {
            return u32::from_str_radix(value, 16)
                .ok()
                .and_then(char::from_u32)
                .map(|ch| Self::Key(Character(ch)))
                .ok_or_else(super::protocol::invalid);
        }
        let key = match text {
            "enter" => Enter,
            "tab" => Tab,
            "back-tab" => BackTab,
            "escape" => Escape,
            "cancel" => Cancel,
            "left" => Left,
            "right" => Right,
            "up" => Up,
            "down" => Down,
            "home" => Home,
            "end" => End,
            "backspace" => Backspace,
            "delete" => Delete,
            "word-left" => WordLeft,
            "word-right" => WordRight,
            "word-backspace" => WordBackspace,
            "kill-start" => KillToStart,
            "kill-end" => KillToEnd,
            "yank" => Yank,
            "previous" => PreviousSuggestion,
            "next" => NextSuggestion,
            "clear" => Clear,
            "eof" => Eof,
            _ => return Err(super::protocol::invalid()),
        };
        Ok(Self::Key(key))
    }
}
