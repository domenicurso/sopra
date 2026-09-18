use crate::syntax;

pub(super) fn insert(buffer: &mut String, cursor: &mut usize, character: char) {
    if is_closer(character) && next_is(buffer, *cursor, character) {
        *cursor += character.len_utf8();
        return;
    }
    if let Some(close) = matching_close(character)
        && can_pair(buffer, *cursor, character)
    {
        buffer.insert(*cursor, character);
        *cursor += character.len_utf8();
        buffer.insert(*cursor, close);
        return;
    }
    buffer.insert(*cursor, character);
    *cursor += character.len_utf8();
}

pub(super) fn should_delete_pair(buffer: &str, start: usize, cursor: usize) -> bool {
    if start >= cursor || cursor >= buffer.len() {
        return false;
    }
    let Some(previous) = buffer[start..cursor].chars().next() else {
        return false;
    };
    let Some(next) = buffer[cursor..].chars().next() else {
        return false;
    };
    matching_close(previous) == Some(next)
}

fn can_pair(buffer: &str, cursor: usize, character: char) -> bool {
    syntax::quote_context(buffer, cursor).is_none()
        && !syntax::comment_context(buffer, cursor)
        && !matches!(character, '\n' | '\r')
}

fn is_closer(character: char) -> bool {
    matches!(character, ')' | ']' | '}' | '\'' | '"')
}

fn matching_close(character: char) -> Option<char> {
    match character {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '\'' => Some('\''),
        '"' => Some('"'),
        _ => None,
    }
}

fn next_is(buffer: &str, cursor: usize, character: char) -> bool {
    buffer[cursor..].starts_with(character)
}
