use crate::input::{Key, MouseEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Character(char),
    Mouse(MouseEvent),
    Ignore,
    Enter,
    Tab,
    BackTab,
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
    PreviousSuggestion,
    NextSuggestion,
    Up,
    Down,
    Home,
    End,
    Cancel,
    Clear,
    PromptWidgets,
    Eof,
}

pub(crate) fn resolve(key: Key) -> Action {
    match key {
        Key::Character(ch) => Action::Character(ch),
        Key::Mouse(event) => Action::Mouse(event),
        Key::Control(ch) => match ch {
            'a' => Action::Home,
            'b' => Action::Left,
            'c' => Action::Cancel,
            'd' => Action::Eof,
            'e' => Action::End,
            'f' => Action::Right,
            'g' => Action::PromptWidgets,
            'k' => Action::KillToEnd,
            'l' => Action::Clear,
            'n' => Action::Down,
            'p' => Action::Up,
            'u' => Action::KillToStart,
            'w' => Action::WordBackspace,
            'y' => Action::Yank,
            _ => Action::Ignore,
        },
        Key::Alt(ch) => match ch {
            'b' => Action::WordLeft,
            'f' => Action::WordRight,
            'd' => Action::WordBackspace,
            _ => Action::Ignore,
        },
        Key::Enter => Action::Enter,
        Key::Tab => Action::Tab,
        Key::BackTab => Action::BackTab,
        Key::Backspace => Action::Backspace,
        Key::Delete => Action::Delete,
        Key::Escape => Action::Escape,
        Key::Left => Action::Left,
        Key::Right => Action::Right,
        Key::Up => Action::Up,
        Key::Down => Action::Down,
        Key::Home => Action::Home,
        Key::End => Action::End,
        Key::AltUp => Action::PreviousSuggestion,
        Key::AltDown => Action::NextSuggestion,
        Key::Ignore => Action::Ignore,
        Key::Paste(_) => Action::Ignore,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_controls_resolve_without_terminal_feature_knowledge() {
        assert_eq!(resolve(Key::Control('g')), Action::PromptWidgets);
        assert_eq!(resolve(Key::Alt('f')), Action::WordRight);
        assert_eq!(resolve(Key::Control('z')), Action::Ignore);
    }
}
