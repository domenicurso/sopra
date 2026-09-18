use super::lex::{DelimiterPair, LexedLine};

pub(super) fn track(
    operator: &str,
    span: usize,
    stack: &mut Vec<(char, usize)>,
    result: &mut LexedLine,
) {
    let Some(character) = operator.chars().next().filter(|_| operator.len() == 1) else {
        return;
    };
    if matches!(character, '(' | '[' | '{') {
        stack.push((character, span));
    } else if let Some(expected) = matching_opener(character) {
        if stack.last().is_some_and(|(open, _)| *open == expected) {
            let (_, open) = stack.pop().expect("delimiter stack has a last item");
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
