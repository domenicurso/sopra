// Metavariables and quoted placeholders may contain spaces; alias separators stay outside them.
pub(crate) fn words(text: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut delimiters = Vec::new();
    let mut quote = None;
    for character in text.chars() {
        if let Some(active) = quote {
            current.push(character);
            if character == active {
                quote = None;
            }
        } else if matches!(character, '\'' | '"') {
            quote = Some(character);
            current.push(character);
        } else if matches!(character, '<' | '[' | '{') {
            delimiters.push(character);
            current.push(character);
        } else if matches!(character, '>' | ']' | '}') {
            delimiters.pop();
            current.push(character);
        } else if character.is_whitespace() && delimiters.is_empty() {
            if !current.is_empty() {
                result.push(std::mem::take(&mut current));
            }
        } else {
            current.push(character);
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    result
}
