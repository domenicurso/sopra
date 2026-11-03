pub(super) fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut angle_depth = 0_usize;
    let mut brace_depth = 0_usize;
    for character in text.chars() {
        if character.is_whitespace() && angle_depth == 0 {
            push_token(&mut tokens, &mut current);
        } else if matches!(character, '[' | ']' | '(' | ')')
            || character == '|' && angle_depth == 0 && brace_depth == 0
        {
            push_token(&mut tokens, &mut current);
            tokens.push(character.to_string());
        } else {
            match character {
                '<' => angle_depth += 1,
                '>' => angle_depth = angle_depth.saturating_sub(1),
                '{' => brace_depth += 1,
                '}' => brace_depth = brace_depth.saturating_sub(1),
                _ => {}
            }
            current.push(character);
        }
    }
    push_token(&mut tokens, &mut current);
    tokens
}

fn push_token(tokens: &mut Vec<String>, current: &mut String) {
    if current.is_empty() {
        return;
    }
    if let Some(value) = current.strip_suffix("...") {
        if !value.is_empty() {
            tokens.push(value.to_string());
        }
        tokens.push("...".to_string());
    } else {
        tokens.push(std::mem::take(current));
    }
    current.clear();
}
