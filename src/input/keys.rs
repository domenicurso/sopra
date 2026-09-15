use std::io::{self, Read};
use std::time::Duration;

use super::{Key, Terminal};

impl Terminal {
    pub(crate) fn read_key(&mut self) -> io::Result<Key> {
        let first = self.read_byte()?;
        match first {
            0x00 => Ok(Key::Eof),
            b'\r' | b'\n' => Ok(Key::Enter),
            b'\t' => Ok(Key::Tab),
            0x03 => Ok(Key::Cancel),
            0x0c => Ok(Key::Clear),
            0x08 | 0x7f => Ok(Key::Backspace),
            0x1b => self.read_escape_sequence(),
            byte if byte.is_ascii() && !byte.is_ascii_control() => Ok(Key::Character(byte as char)),
            byte if byte >= 0x80 => self.read_utf8(byte),
            _ => Ok(Key::Eof),
        }
    }

    fn read_byte(&mut self) -> io::Result<u8> {
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
            return Ok(Key::Escape);
        }
        let mut sequence = Vec::with_capacity(8);
        for _ in 0..8 {
            if !self.poll(Duration::from_millis(4))? {
                break;
            }
            let byte = self.read_byte()?;
            sequence.push(byte);
            if byte.is_ascii_alphabetic() || byte == b'~' {
                break;
            }
        }
        Ok(match sequence.last().copied() {
            Some(b'A') => Key::Up,
            Some(b'B') => Key::Down,
            Some(b'C') => Key::Right,
            Some(b'D') => Key::Left,
            Some(b'H') => Key::Home,
            Some(b'F') => Key::End,
            Some(b'~') if sequence.starts_with(b"3") => Key::Delete,
            _ => Key::Escape,
        })
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
