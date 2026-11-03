use super::Key;

// Disambiguation changes control and Alt keys as well as Shift-Enter, so decode
// those here before passing the same physical keys to the action bindings.
pub(super) fn decode(sequence: &[u8]) -> Key {
    let Ok(text) = std::str::from_utf8(sequence) else {
        return Key::Ignore;
    };
    let mut fields = text.split(';');
    let Some(code) = fields.next().and_then(|code| code.parse::<u32>().ok()) else {
        return Key::Ignore;
    };
    let modifiers = match fields.next() {
        None => 1,
        Some(value) => match value.parse::<u8>() {
            Ok(value) => value,
            Err(_) => return Key::Ignore,
        },
    };
    if fields.next().is_some() {
        return Key::Ignore;
    }
    match (code, modifiers) {
        (13, 2) => Key::ShiftEnter,
        (13, 1) => Key::Enter,
        (9, 1) => Key::Tab,
        (9, 2) => Key::BackTab,
        (27, 1) => Key::Escape,
        (127, 1 | 5) | (104, 5) => Key::Backspace,
        (105, 5) => Key::Tab,
        (106 | 109, 5) => Key::Enter,
        (91, 5) => Key::Escape,
        (97..=122, 5) => char::from_u32(code).map_or(Key::Ignore, Key::Control),
        (32..=126, 3) => char::from_u32(code).map_or(Key::Ignore, Key::Alt),
        _ => Key::Ignore,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enhanced_keyboard_preserves_editing_shortcuts() {
        for (sequence, expected) in [
            ("13;2", Key::ShiftEnter),
            ("13", Key::Enter),
            ("99;5", Key::Control('c')),
            ("114;5", Key::Control('r')),
            ("103;3", Key::Alt('g')),
            ("27", Key::Escape),
            ("9;2", Key::BackTab),
            ("127", Key::Backspace),
            ("109;5", Key::Enter),
            ("105;5", Key::Tab),
            ("104;5", Key::Backspace),
            ("91;5", Key::Escape),
            ("13;2:3", Key::Ignore),
            ("invalid", Key::Ignore),
            ("13;6", Key::Ignore),
        ] {
            assert_eq!(decode(sequence.as_bytes()), expected, "{sequence}");
        }
    }
}
