use std::time::{Duration, Instant};

#[derive(Default)]
pub(super) struct Input {
    pending: Vec<u8>,
    paste: bool,
    last: Option<Instant>,
}
impl Input {
    pub(super) fn feed(&mut self, bytes: &[u8]) -> Vec<Vec<u8>> {
        if !bytes.is_empty() {
            self.last = Some(Instant::now());
            self.pending.extend_from_slice(bytes);
        }
        let mut packets = Vec::new();
        while !self.pending.is_empty() {
            let Some(length) = self.length() else {
                break;
            };
            let packet = self.pending.drain(..length).collect::<Vec<_>>();
            if packet == b"\x1b[200~" {
                self.paste = true;
            }
            if packet == b"\x1b[201~" {
                self.paste = false;
            }
            packets.push(packet);
        }
        packets
    }
    fn length(&self) -> Option<usize> {
        if self.paste {
            return match self
                .pending
                .windows(6)
                .position(|window| window == b"\x1b[201~")
            {
                Some(0) => Some(6),
                Some(index) => Some(index),
                None if self.pending.len() > 5 => Some(self.pending.len() - 5),
                _ => None,
            };
        }
        if self.pending[0] != 0x1b {
            let length = match self.pending[0] {
                0..=127 => 1,
                192..=223 => 2,
                224..=239 => 3,
                240..=247 => 4,
                _ => return Some(1),
            };
            return (self.pending.len() >= length).then_some(length);
        }
        if self.pending.len() == 1 {
            return self
                .last
                .filter(|time| time.elapsed() > Duration::from_millis(25))
                .map(|_| 1);
        }
        self.escape_length()
    }
    fn escape_length(&self) -> Option<usize> {
        match self.pending[1] {
            b'[' => self.pending[2..]
                .iter()
                .position(|byte| (0x40..=0x7e).contains(byte))
                .map(|index| index + 3),
            b']' | b'P' => self
                .pending
                .iter()
                .position(|byte| *byte == 7)
                .map(|index| index + 1)
                .or_else(|| {
                    self.pending
                        .windows(2)
                        .position(|bytes| bytes == b"\x1b\\")
                        .map(|index| index + 2)
                }),
            b'O' if self.pending.len() < 3 => None,
            b'O' => Some(3),
            _ => Some(2),
        }
        .or_else(|| (self.pending.len() > 65536).then_some(self.pending.len()))
    }
    #[cfg(test)]
    pub(super) fn pasting(&self) -> bool {
        self.paste
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Mouse {
    pub(super) button: u16,
    pub(super) column: u16,
    pub(super) row: u16,
    pub(super) release: bool,
}
impl Mouse {
    pub(super) fn parse(bytes: &[u8]) -> Option<Self> {
        let text = std::str::from_utf8(bytes.strip_prefix(b"\x1b[<")?).ok()?;
        let release = text.ends_with('m');
        if !release && !text.ends_with('M') {
            return None;
        }
        let mut parts = text[..text.len() - 1].split(';');
        let button = parts.next()?.parse().ok()?;
        let column = parts.next()?.parse::<u16>().ok()?.checked_sub(1)?;
        let row = parts.next()?.parse::<u16>().ok()?.checked_sub(1)?;
        (parts.next().is_none()).then_some(Self {
            button,
            column,
            row,
            release,
        })
    }
    pub(super) fn encode(self, encoding: vt100::MouseProtocolEncoding) -> Vec<u8> {
        if encoding == vt100::MouseProtocolEncoding::Sgr {
            return format!(
                "\x1b[<{};{};{}{}",
                self.button,
                self.column + 1,
                self.row + 1,
                if self.release { 'm' } else { 'M' }
            )
            .into_bytes();
        }
        let button = if self.release { 3 } else { self.button };
        if encoding == vt100::MouseProtocolEncoding::Utf8 {
            let mut output = b"\x1b[M".to_vec();
            for value in [
                u32::from(button) + 32,
                u32::from(self.column) + 33,
                u32::from(self.row) + 33,
            ] {
                if let Some(character) = char::from_u32(value) {
                    output.extend_from_slice(character.encode_utf8(&mut [0; 4]).as_bytes());
                }
            }
            return output;
        }
        match (
            u8::try_from(u32::from(button) + 32),
            u8::try_from(u32::from(self.column) + 33),
            u8::try_from(u32::from(self.row) + 33),
        ) {
            (Ok(button), Ok(column), Ok(row)) => vec![27, b'[', b'M', button, column, row],
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fragmented_mouse_sequences_and_paste_preserve_literal_shortcuts() {
        let mut input = Input::default();
        assert!(input.feed(b"\x1b[<64;2;").is_empty());
        let packets = input.feed(b"4M");
        assert_eq!(
            Mouse::parse(&packets[0]).map(|mouse| (mouse.button, mouse.row)),
            Some((64, 3))
        );
        assert_eq!(input.feed(b"\x1b[200~"), [b"\x1b[200~".to_vec()]);
        assert!(input.pasting());
        let packets = input.feed(b"hello\x1bh\x1b[201~");
        assert_eq!(packets.concat(), b"hello\x1bh\x1b[201~");
        assert!(!input.pasting());
    }
}
