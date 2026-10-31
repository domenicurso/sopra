use super::PromptSpan;
use ratatui::style::Style;
use std::io;

#[path = "template_render.rs"]
mod rendering;
#[cfg(test)]
#[path = "template_tests.rs"]
mod tests;

#[derive(Clone, Debug)]
pub(crate) enum Piece {
    Text(PromptSpan),
    Value { name: String, style: Style },
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Template {
    pieces: Vec<Piece>,
}

impl Template {
    pub(crate) fn compile(source: &str) -> io::Result<Self> {
        let mut result = Self::default();
        let mut rest = source;
        let mut style = Style::default();
        let mut stack = Vec::new();
        while !rest.is_empty() {
            if let Some(next) = rest.strip_prefix("$$") {
                result.text("$", style);
                rest = next;
            } else if let Some(next) = rest.strip_prefix("[[") {
                result.text("[", style);
                rest = next;
            } else if let Some(length) = tag(rest, &mut style, &mut stack)? {
                rest = &rest[length..];
            } else if let Some(name) = reference(rest) {
                result.pieces.push(Piece::Value {
                    name: name.into(),
                    style,
                });
                rest = &rest[name.len() + 1..];
            } else {
                let ch = rest.chars().next().ok_or_else(|| invalid("missing text"))?;
                if ch == '\0' {
                    return Err(invalid("templates cannot contain NUL"));
                }
                result.text(&rest[..ch.len_utf8()], style);
                rest = &rest[ch.len_utf8()..];
            }
        }
        if !stack.is_empty() {
            return Err(invalid("unclosed style tag"));
        }
        Ok(result)
    }

    pub(crate) fn references(&self) -> impl Iterator<Item = &str> {
        self.pieces.iter().filter_map(|piece| match piece {
            Piece::Value { name, .. } => Some(name.as_str()),
            _ => None,
        })
    }

    fn text(&mut self, text: &str, style: Style) {
        if let Some(Piece::Text(last)) = self
            .pieces
            .last_mut()
            .filter(|piece| matches!(piece, Piece::Text(span) if span.style == style))
        {
            last.text.push_str(text);
        } else {
            self.pieces.push(Piece::Text(PromptSpan {
                text: text.into(),
                style,
            }));
        }
    }
}

fn reference(source: &str) -> Option<&str> {
    let rest = source.strip_prefix('$')?;
    let end = rest
        .find(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .unwrap_or(rest.len());
    let name = &rest[..end];
    (!name.is_empty() && !name.starts_with(|ch: char| ch.is_ascii_digit())).then_some(name)
}

fn tag(
    source: &str,
    style: &mut Style,
    stack: &mut Vec<(String, Style)>,
) -> io::Result<Option<usize>> {
    let Some(rest) = source.strip_prefix('[') else {
        return Ok(None);
    };
    let Some(end) = rest.find(']') else {
        return Ok(None);
    };
    let name = &rest[..end];
    if let Some(close) = name.strip_prefix('/') {
        let (open, previous) = stack
            .pop()
            .ok_or_else(|| invalid("unexpected closing style tag"))?;
        if close != open {
            return Err(invalid(&format!("expected [/{open}], found [/{close}]")));
        }
        *style = previous;
    } else if let Some(overlay) = super::style::attributes(name) {
        stack.push((name.into(), *style));
        *style = style.patch(overlay);
    } else {
        return Ok(None);
    }
    Ok(Some(end + 2))
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
