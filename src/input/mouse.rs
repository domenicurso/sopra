use super::CursorPosition;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MouseKind {
    Press,
    Drag,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MouseEvent {
    pub(crate) kind: MouseKind,
    pub(crate) position: CursorPosition,
}

pub(super) fn parse_sgr(sequence: &[u8]) -> Option<MouseEvent> {
    let (&terminator, parameters) = sequence.split_last()?;
    let text = std::str::from_utf8(parameters).ok()?;
    let mut fields = text.split(';');
    let button: u16 = fields.next()?.parse().ok()?;
    let column: u16 = fields.next()?.parse().ok()?;
    let row: u16 = fields.next()?.parse().ok()?;
    if fields.next().is_some() || column == 0 || row == 0 || button & 0b1100_0011 != 0 {
        return None;
    }
    let kind = match (terminator, button & 32 != 0) {
        (b'm', _) => MouseKind::Release,
        (b'M', true) => MouseKind::Drag,
        (b'M', false) => MouseKind::Press,
        _ => return None,
    };
    Some(MouseEvent {
        kind,
        position: CursorPosition {
            column: column - 1,
            row: row - 1,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_sgr_mouse_and_rejects_other_buttons() {
        assert_eq!(
            parse_sgr(b"0;120;35M"),
            Some(MouseEvent {
                kind: MouseKind::Press,
                position: CursorPosition {
                    column: 119,
                    row: 34
                }
            })
        );
        assert_eq!(
            parse_sgr(b"32;2;4M").map(|event| event.kind),
            Some(MouseKind::Drag)
        );
        assert_eq!(
            parse_sgr(b"0;2;4m").map(|event| event.kind),
            Some(MouseKind::Release)
        );
        assert_eq!(parse_sgr(b"64;2;4M"), None);
        assert_eq!(parse_sgr(b"0;0;4M"), None);
    }
}
