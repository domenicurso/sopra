#[cfg(test)]
use super::lex::{is_assignment, lex};

#[cfg(test)]
pub(super) fn command_position(line: &str, cursor: usize) -> bool {
    let cursor = cursor.min(line.len());
    let prefix = &line[..cursor];
    let lexed = lex(prefix);
    if let Some(word) = lexed.words.last()
        && prefix.ends_with(&word.text)
    {
        return word.command_expected && !is_assignment(&word.text);
    }
    lexed.final_state.command_expected
}

pub(super) fn quote_context(line: &str, cursor: usize) -> Option<char> {
    let mut quote = None;
    let mut escaped = false;
    let mut word_start = true;
    for (index, character) in line.char_indices() {
        if index >= cursor {
            break;
        }
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' && quote != Some('\'') {
            escaped = true;
            continue;
        }
        if let Some(active) = quote {
            if character == active {
                quote = None;
            }
            continue;
        }
        if character == '#' && word_start {
            break;
        }
        if matches!(character, '\'' | '"') {
            quote = Some(character);
            word_start = false;
        } else {
            word_start = character.is_whitespace();
        }
    }
    quote
}

pub(super) fn comment_context(line: &str, cursor: usize) -> bool {
    let mut quote = None;
    let mut escaped = false;
    let mut word_start = true;
    for (index, character) in line.char_indices() {
        if index >= cursor {
            break;
        }
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' && quote != Some('\'') {
            escaped = true;
            continue;
        }
        if let Some(active) = quote {
            if character == active {
                quote = None;
            }
            continue;
        }
        if character == '#' && word_start {
            return true;
        }
        if matches!(character, '\'' | '"') {
            quote = Some(character);
            word_start = false;
        } else {
            word_start = character.is_whitespace() || matches!(character, '|' | '&' | ';');
        }
    }
    false
}
