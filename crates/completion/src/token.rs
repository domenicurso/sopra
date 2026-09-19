pub fn token_range(line: &str, cursor: usize) -> (usize, usize) {
    let cursor = cursor.min(line.len());
    let mut ranges = Vec::new();
    let mut start = None;
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' && quote != Some('\'') {
            start.get_or_insert(index);
            escaped = true;
            continue;
        }
        if let Some(active) = quote {
            if character == active {
                quote = None;
            }
            start.get_or_insert(index);
            continue;
        }
        if matches!(character, '\'' | '"') {
            start.get_or_insert(index);
            quote = Some(character);
        } else if character.is_whitespace() || is_operator(character) {
            if let Some(token_start) = start.take() {
                ranges.push((token_start, index));
            }
        } else {
            start.get_or_insert(index);
        }
    }
    if let Some(token_start) = start {
        ranges.push((token_start, line.len()));
    }
    ranges
        .into_iter()
        .find(|(start, end)| {
            *start <= cursor
                && (cursor < *end
                    || cursor == *end
                        && (cursor == line.len()
                            || line[cursor..].chars().next().is_some_and(is_operator)))
        })
        .unwrap_or((cursor, cursor))
}

fn is_operator(character: char) -> bool {
    matches!(character, '|' | '&' | ';' | '(' | ')' | '<' | '>')
}

#[cfg(test)]
mod tests {
    use super::token_range;

    #[test]
    fn quoted_whitespace_stays_inside_one_token() {
        assert_eq!(token_range("echo \"hello world\"", 12), (5, 18));
    }

    #[test]
    fn escaped_operators_stay_inside_one_token() {
        assert_eq!(token_range("echo foo\\|bar", 13), (5, 13));
    }
}
