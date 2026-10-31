use super::{Key, Terminal};
use std::{io, time::Duration};

impl Terminal {
    pub(super) fn read_paste(&mut self) -> io::Result<Key> {
        let mut bytes = Vec::new();
        loop {
            if bytes.len() > 8 * 1024 * 1024 {
                return Err(io::Error::other("paste exceeds 8 MiB"));
            }
            if !self.poll(Duration::from_secs(3))? {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "paste terminator missing",
                ));
            }
            bytes.push(self.read_byte()?);
            if bytes.ends_with(b"\x1b[201~") {
                bytes.truncate(bytes.len() - 6);
                return Ok(Key::Paste(normalize(&bytes)?));
            }
        }
    }
}
fn normalize(bytes: &[u8]) -> io::Result<String> {
    let text = std::str::from_utf8(bytes).map_err(io::Error::other)?;
    Ok(text
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .chars()
        .filter(|ch| !ch.is_control() || matches!(ch, '\n' | '\t'))
        .collect())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pastes_are_literal_multiline_unicode_without_terminal_controls() -> io::Result<()> {
        assert_eq!(
            normalize("echo 'é'\r\nnext\r\x1b\x03".as_bytes())?,
            "echo 'é'\nnext\n"
        );
        assert!(normalize(&[255]).is_err());
        Ok(())
    }
}
