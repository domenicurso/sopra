use std::io::{self, Read};
use std::time::Duration;

use super::{Key, Terminal};

impl Terminal {
    pub(crate) fn read_key(&mut self) -> io::Result<Key> {
        let first = self.read_byte()?;
        match first {
            b'\r' | b'\n' => Ok(Key::Enter),
            b'\t' => Ok(Key::Tab),
            byte @ 0x01..=0x1a if byte != 0x08 && byte != 0x1b => {
                Ok(Key::Control((b'a' + byte - 1) as char))
            }
            0x08 | 0x7f => Ok(Key::Backspace),
            0x1b => self.read_escape_sequence(),
            byte if byte.is_ascii() && !byte.is_ascii_control() => Ok(Key::Character(byte as char)),
            byte if byte >= 0x80 => self.read_utf8(byte),
            _ => Ok(Key::Ignore),
        }
    }

    pub(super) fn read_byte(&mut self) -> io::Result<u8> {
        if let Some(byte) = self.pending_input.pop_front() {
            return Ok(byte);
        }
        let mut byte = [0_u8; 1];
        self.tty.read_exact(&mut byte)?;
        Ok(byte[0])
    }

    fn read_escape_sequence(&mut self) -> io::Result<Key> {
        if !self.poll(Duration::from_millis(8))? {
            return Ok(Key::Escape);
        }
        let prefix = self.read_byte()?;
        if prefix != b'[' && prefix != b'O' {
            return Ok(if prefix.is_ascii_alphabetic() {
                Key::Alt(prefix as char)
            } else {
                Key::Escape
            });
        }
        if prefix == b'[' && self.poll(Duration::from_millis(4))? {
            let first = self.read_byte()?;
            if first == b'<' {
                return self.read_mouse_sequence();
            }
            self.pending_input.push_front(first);
        }
        let mut sequence = Vec::with_capacity(8);
        for _ in 0..32 {
            if !self.poll(Duration::from_millis(4))? {
                break;
            }
            let byte = self.read_byte()?;
            sequence.push(byte);
            if byte.is_ascii_alphabetic() || byte == b'~' {
                break;
            }
        }
        if prefix == b'[' && sequence == b"200~" {
            return self.read_paste();
        }
        Ok(decode_key(prefix, &sequence))
    }

    fn read_mouse_sequence(&mut self) -> io::Result<Key> {
        let mut sequence = Vec::with_capacity(16);
        for _ in 0..32 {
            if !self.poll(Duration::from_millis(8))? {
                return Ok(Key::Ignore);
            }
            let byte = self.read_byte()?;
            sequence.push(byte);
            if byte == b'M' || byte == b'm' {
                return Ok(super::mouse::parse_sgr(&sequence).map_or(Key::Ignore, Key::Mouse));
            }
        }
        Ok(Key::Ignore)
    }

    fn read_utf8(&mut self, first: u8) -> io::Result<Key> {
        let expected = if first & 0b1110_0000 == 0b1100_0000 {
            2
        } else if first & 0b1111_0000 == 0b1110_0000 {
            3
        } else if first & 0b1111_1000 == 0b1111_0000 {
            4
        } else {
            return Ok(Key::Character('�'));
        };
        let mut bytes = vec![first];
        for _ in 1..expected {
            if !self.poll(Duration::from_millis(50))? {
                return Ok(Key::Character('�'));
            }
            bytes.push(self.read_byte()?);
        }
        Ok(std::str::from_utf8(&bytes)
            .ok()
            .and_then(|text| text.chars().next())
            .map(Key::Character)
            .unwrap_or(Key::Character('�')))
    }
}

fn decode_key(prefix: u8, sequence: &[u8]) -> Key {
    if prefix == b'[' && sequence.ends_with(b"u") {
        return super::keyboard::decode(&sequence[..sequence.len() - 1]);
    }
    match sequence.last().copied() {
        Some(b'Z') if prefix == b'[' => Key::BackTab,
        Some(b'~') if sequence == b"27;2;13~" => Key::ShiftEnter,
        Some(b'A') if sequence == b"1;3A" => Key::AltUp,
        Some(b'B') if sequence == b"1;3B" => Key::AltDown,
        Some(b'A') => Key::Up,
        Some(b'B') => Key::Down,
        Some(b'C') => Key::Right,
        Some(b'D') => Key::Left,
        Some(b'H') => Key::Home,
        Some(b'F') => Key::End,
        Some(b'~') if sequence.starts_with(b"3") => Key::Delete,
        _ => Key::Escape,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modified_enter_supports_csi_u_and_modify_other_keys() {
        assert_eq!(decode_key(b'[', b"13;2u"), Key::ShiftEnter);
        assert_eq!(decode_key(b'[', b"27;2;13~"), Key::ShiftEnter);
        assert_eq!(decode_key(b'[', b"13;6u"), Key::Ignore);
    }
}
