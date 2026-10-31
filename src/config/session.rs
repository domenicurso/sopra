use std::io;
use toml_edit::DocumentMut;

#[derive(Clone, Copy)]
pub(crate) struct Session {
    pub(crate) max_blocks: usize,
    pub(crate) scrollback_lines: usize,
    pub(crate) max_output_bytes: usize,
}
impl Session {
    pub(super) fn parse(document: &DocumentMut) -> io::Result<Self> {
        let number = |key: &str| {
            document
                .get("session")
                .and_then(|section| section.get(key))
                .and_then(|item| item.as_integer())
                .and_then(|number| usize::try_from(number).ok())
                .filter(|number| *number > 0)
                .ok_or_else(|| {
                    io::Error::other(format!("session.{key} must be a positive integer"))
                })
        };
        Ok(Self {
            max_blocks: number("max_blocks")?,
            scrollback_lines: number("scrollback_lines")?,
            max_output_bytes: number("max_output_bytes")?,
        })
    }
}
