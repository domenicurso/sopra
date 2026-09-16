use super::{Rgb, TerminalPalette};

pub(super) fn parse_responses(bytes: &[u8]) -> (TerminalPalette, Vec<u8>) {
    let mut palette = TerminalPalette::default();
    let mut pending = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let Some(end) = osc_end(bytes, index) else {
            pending.push(bytes[index]);
            index += 1;
            continue;
        };
        if let Some((code, color)) = parse_osc(&bytes[index..end])
            && matches!(code, 11 | 12)
        {
            if code == 11 {
                palette.background = Some(color);
            } else {
                palette.cursor = Some(color);
            }
            index = end;
        } else {
            pending.extend_from_slice(&bytes[index..end]);
            index = end;
        }
    }
    (palette, pending)
}

fn osc_end(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&0x1b) || bytes.get(start + 1) != Some(&b']') {
        return None;
    }
    for index in start + 2..bytes.len() {
        if bytes[index] == 0x07 {
            return Some(index + 1);
        }
        if bytes[index] == 0x1b && bytes.get(index + 1) == Some(&b'\\') {
            return Some(index + 2);
        }
    }
    None
}

fn parse_osc(bytes: &[u8]) -> Option<(u8, Rgb)> {
    let payload = bytes.get(2..)?;
    let payload = payload
        .strip_suffix(&[0x07])
        .or_else(|| payload.strip_suffix(&[0x1b, b'\\']))?;
    let separator = payload.iter().position(|byte| *byte == b';')?;
    let code = std::str::from_utf8(&payload[..separator])
        .ok()?
        .parse()
        .ok()?;
    let color = parse_color(&payload[separator + 1..])?;
    Some((code, color))
}

fn parse_color(value: &[u8]) -> Option<Rgb> {
    if let Some(value) = value.strip_prefix(b"rgb:") {
        let mut components = value.split(|byte| *byte == b'/');
        return Some(Rgb {
            red: parse_component(components.next()?)?,
            green: parse_component(components.next()?)?,
            blue: parse_component(components.next()?)?,
        });
    }
    let value = value.strip_prefix(b"#")?;
    if value.len() != 6 {
        return None;
    }
    Some(Rgb {
        red: parse_hex(&value[..2])?,
        green: parse_hex(&value[2..4])?,
        blue: parse_hex(&value[4..])?,
    })
}

fn parse_component(value: &[u8]) -> Option<u8> {
    if !(1..=4).contains(&value.len()) {
        return None;
    }
    let number = value.iter().try_fold(0_u32, |total, byte| {
        Some(total * 16 + hex_value(*byte)? as u32)
    })?;
    let maximum = (1_u32 << (value.len() * 4)) - 1;
    Some(((number * 255 + maximum / 2) / maximum) as u8)
}

fn parse_hex(value: &[u8]) -> Option<u8> {
    value
        .iter()
        .try_fold(0_u8, |total, byte| Some(total * 16 + hex_value(*byte)?))
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use ratatui::style::Modifier;

    use super::parse_responses;

    #[test]
    fn parses_rgb_and_hex_terminal_responses() {
        let bytes = b"\x1b]11;rgb:0000/8000/ffff\x1b\\\x1b]12;#123456\x07";
        let (palette, pending) = parse_responses(bytes);
        assert!(pending.is_empty());
        assert_eq!(palette.background.map(|color| color.green), Some(128));
        assert_eq!(palette.cursor.map(|color| color.blue), Some(0x56));
    }

    #[test]
    fn preserves_non_response_bytes_for_the_key_reader() {
        let (palette, pending) = parse_responses(b"x\x1b]11;#010203\x1b\\y");
        assert_eq!(pending, b"xy");
        assert!(
            palette
                .cursor_style(1.0)
                .add_modifier
                .contains(Modifier::REVERSED)
        );
    }
}
