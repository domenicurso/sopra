use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub(super) enum Event {
    Begin { command: String, cwd: PathBuf },
    End(i32),
    Executing,
    Prompt(u16),
    Editing { secondary: bool },
}

pub(super) fn event(params: &[&[u8]]) -> Option<Event> {
    if params.first()? != b"777" || *params.get(1)? != b"sopra" {
        return None;
    }
    match *params.get(2)? {
        b"executing" if params.len() == 3 => Some(Event::Executing),
        b"begin" if params.len() == 5 => Some(Event::Begin {
            command: decode(params[3])?,
            cwd: decode(params[4])?.into(),
        }),
        b"editing" if params.len() == 4 => match params[3] {
            b"start" => Some(Event::Editing { secondary: false }),
            b"cont" => Some(Event::Editing { secondary: true }),
            _ => None,
        },
        b"end" if params.len() == 4 => Some(Event::End(
            std::str::from_utf8(params[3]).ok()?.parse().ok()?,
        )),
        _ => None,
    }
}
fn decode(bytes: &[u8]) -> Option<String> {
    if !bytes.is_ascii() || bytes.len() > 524288 || !bytes.len().is_multiple_of(2) {
        return None;
    }
    let text = std::str::from_utf8(bytes).ok()?;
    let bytes = (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    let value = String::from_utf8(bytes).ok()?;
    (!value.contains('\0')).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_decodes_without_interpreting_command_control_sequences() {
        assert_eq!(
            event(&[
                b"777",
                b"sopra",
                b"begin",
                b"6563686f2027c3a927",
                b"2f746d70"
            ]),
            Some(Event::Begin {
                command: "echo 'é'".into(),
                cwd: "/tmp".into()
            })
        );
        for malformed in [b"a".as_slice(), b"zz", b"c0ff", b"00", "aéa".as_bytes()] {
            assert!(decode(malformed).is_none());
        }
        assert!(event(&[b"777", b"other", b"end", b"0"]).is_none());
    }
}
