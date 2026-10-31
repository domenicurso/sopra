use std::io;
use toml_edit::DocumentMut;

#[derive(Clone, Copy)]
pub(crate) struct Session {
    pub(crate) scrollback_lines: usize,
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
            scrollback_lines: number("scrollback_lines")?,
        })
    }
}
