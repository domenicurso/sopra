use super::tokens::{DelimiterPair, LexedLine};

pub(super) fn track(
    operator: &str,
    span: usize,
    stack: &mut Vec<(char, usize)>,
    result: &mut LexedLine,
) {
    let character = match operator {
        "[[" => '[',
        "]]" => ']',
        _ if operator.len() == 1 => operator.chars().next().unwrap_or(' '),
        _ => return,
    };
    if matches!(character, '(' | '[' | '{') {
        stack.push((character, span));
    } else if let Some(expected) = matching_opener(character) {
        if stack.last().is_some_and(|(open, _)| *open == expected) {
            let Some((_, open)) = stack.pop() else { return };
            result.delimiter_pairs.push(DelimiterPair {
                open,
                close: Some(span),
            });
        } else {
            result.unmatched_delimiters.push(span);
        }
    }
}

fn matching_opener(character: char) -> Option<char> {
    match character {
        ')' => Some('('),
        ']' => Some('['),
        '}' => Some('{'),
        _ => None,
    }
}

pub(super) fn parameter_close(line: &str, start: usize) -> Option<usize> {
    let mut depth = 1;
    for (offset, character) in line[start..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
        if depth == 0 {
            return Some(start + offset);
        }
    }
    None
}
