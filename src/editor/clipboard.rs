use super::EditorState;
use crate::input::Terminal;
use std::io;

impl EditorState {
    pub(super) fn copy_selection(&self, terminal: &mut Terminal) -> io::Result<bool> {
        let Some(range) = self.text_selection.range() else {
            return Ok(false);
        };
        let encoded = encode_base64(self.buffer[range].as_bytes());
        terminal.write_all(format!("\x1b]52;c;{encoded}\x1b\\").as_bytes())?;
        terminal.flush()?;
        Ok(true)
    }
}

fn encode_base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let value = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        output.push(ALPHABET[((value >> 18) & 63) as usize] as char);
        output.push(ALPHABET[((value >> 12) & 63) as usize] as char);
        output.push(if chunk.len() > 1 {
            ALPHABET[((value >> 6) & 63) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            ALPHABET[(value & 63) as usize] as char
        } else {
            '='
        });
    }
    output
}

#[cfg(test)]
mod tests {
    use super::encode_base64;
    #[test]
    fn encodes_clipboard_text_with_padding() {
        assert_eq!(encode_base64(b""), "");
        assert_eq!(encode_base64(b"a"), "YQ==");
        assert_eq!(encode_base64(b"ab"), "YWI=");
        assert_eq!(encode_base64("café".as_bytes()), "Y2Fmw6k=");
    }
}
