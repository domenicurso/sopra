pub(super) fn command_position(line: &str, cursor: usize) -> bool {
    let prefix = &line[..byte_cursor(line, cursor)];
    let mut command_expected = true;
    let mut redirect_expected = false;
    let mut word = String::new();
    for character in prefix.chars() {
        if character.is_whitespace() {
            consume_word(&mut word, &mut command_expected, &mut redirect_expected);
            continue;
        }
        if is_operator(character) {
            consume_word(&mut word, &mut command_expected, &mut redirect_expected);
            consume_operator(character, &mut command_expected, &mut redirect_expected);
        } else {
            word.push(character);
        }
    }
    command_expected && (word.is_empty() || !is_assignment(&word))
}

pub(super) fn quote_context(line: &str, cursor: usize) -> Option<char> {
    let mut quote = None;
    let mut escaped = false;
    let mut word_start = true;
    for (index, character) in line.char_indices() {
        if index >= byte_cursor(line, cursor) {
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

fn consume_word(word: &mut String, command_expected: &mut bool, redirect_expected: &mut bool) {
    if word.is_empty() {
        return;
    }
    if *redirect_expected {
        *redirect_expected = false;
    } else if *command_expected && !is_assignment(word) {
        *command_expected = false;
    }
    word.clear();
}

fn consume_operator(character: char, command_expected: &mut bool, redirect_expected: &mut bool) {
    *redirect_expected = matches!(character, '<' | '>');
    if matches!(character, '|' | '&' | ';') {
        *command_expected = true;
    } else if !*redirect_expected {
        *command_expected = false;
    }
}

fn is_assignment(word: &str) -> bool {
    let Some((name, _)) = word.split_once('=') else {
        return false;
    };
    !name.is_empty()
        && name.chars().enumerate().all(|(index, character)| {
            if index == 0 {
                character == '_' || character.is_ascii_alphabetic()
            } else {
                character == '_' || character.is_ascii_alphanumeric()
            }
        })
}

fn is_operator(character: char) -> bool {
    matches!(character, '|' | '&' | ';' | '<' | '>')
}

fn byte_cursor(line: &str, cursor: usize) -> usize {
    line.char_indices()
        .nth(cursor)
        .map_or(line.len(), |(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::{command_position, quote_context};

    #[test]
    fn command_position_handles_assignments_and_pipelines() {
        assert!(command_position("FOO=bar ", 8));
        assert!(command_position("echo hi | ", 10));
        assert!(!command_position("echo hi", 7));
    }

    #[test]
    fn quote_context_ignores_escaped_quotes() {
        assert_eq!(quote_context("echo \"hi", 8), Some('"'));
        assert_eq!(quote_context("echo \\\"hi", 9), None);
    }
}
