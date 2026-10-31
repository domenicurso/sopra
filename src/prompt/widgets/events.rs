use crate::actions::Action as Key;
use sopra::{Event, Key as WidgetKey};

pub(super) fn event(key: Key) -> Option<Event> {
    use WidgetKey as W;
    let key = match key {
        Key::PromptWidgets => return Some(Event::Activate),
        Key::Character(ch) => W::Character(ch),
        Key::Enter => W::Enter,
        Key::Tab => W::Tab,
        Key::BackTab => W::BackTab,
        Key::Escape => W::Escape,
        Key::Cancel => W::Cancel,
        Key::Left => W::Left,
        Key::Right => W::Right,
        Key::Up => W::Up,
        Key::Down => W::Down,
        Key::Home => W::Home,
        Key::End => W::End,
        Key::Backspace => W::Backspace,
        Key::Delete => W::Delete,
        Key::WordLeft => W::WordLeft,
        Key::WordRight => W::WordRight,
        Key::WordBackspace => W::WordBackspace,
        Key::KillToStart => W::KillToStart,
        Key::KillToEnd => W::KillToEnd,
        Key::Yank => W::Yank,
        Key::PreviousSuggestion => W::PreviousSuggestion,
        Key::NextSuggestion => W::NextSuggestion,
        Key::Clear => W::Clear,
        Key::Eof => W::Eof,
        Key::Mouse(_) | Key::Ignore | Key::ToggleMouseEditing | Key::SearchHistory => return None,
    };
    Some(Event::Key(key))
}
