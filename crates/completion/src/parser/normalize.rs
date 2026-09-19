#[allow(clippy::while_let_loop)]
pub(crate) fn terminal_text(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    loop {
        let character = match chars.next() {
            Some(character) => character,
            None => break,
        };
        match character {
            '\x1b' => skip_escape(&mut chars),
            '\x08' => {
                output.pop();
            }
            '\r' => {}
            _ => output.push(character),
        }
    }
    output
}

#[allow(clippy::while_let_loop)]
fn skip_escape(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    let Some(next) = chars.next() else {
        return;
    };
    if next == ']' {
        while let Some(character) = chars.next() {
            if character == '\x07' {
                break;
            }
            if character == '\x1b' && chars.peek() == Some(&'\\') {
                chars.next();
                break;
            }
        }
        return;
    }
    if next != '[' {
        return;
    }
    loop {
        let character = match chars.next() {
            Some(character) => character,
            None => break,
        };
        if character.is_ascii_alphabetic() || character == '~' {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::terminal_text;

    #[test]
    fn removes_terminal_sequences_and_overstrikes() {
        assert_eq!(terminal_text("\x1b[31mred\x1b[0m\x08!"), "re!");
    }
}
