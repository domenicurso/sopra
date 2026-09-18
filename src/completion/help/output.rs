pub(super) fn clean_terminal_text(text: &str) -> String {
    let mut output = String::new();
    let mut escape = false;
    for character in text.chars() {
        if escape {
            if character.is_ascii_alphabetic() {
                escape = false;
            }
            continue;
        }
        if character == '\x1b' {
            escape = true;
        } else if character == '\x08' {
            output.pop();
        } else {
            output.push(character);
        }
    }
    output
}
