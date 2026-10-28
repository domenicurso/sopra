#[path = "config_boolean.rs"]
mod boolean;

use boolean::read_bool;
use std::{fs, io, path::Path};

use toml_edit::{DocumentMut, Item};

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct PromptConfig {
    pub(crate) cursor_animation: Option<bool>,
    pub(crate) syntax_highlighting: Option<bool>,
    pub(crate) completion_enabled: Option<bool>,
    pub(crate) left: Option<String>,
    pub(crate) right: Option<String>,
    pub(crate) transient: Option<String>,
    pub(crate) secondary: Option<String>,
    pub(crate) clock_format: Option<String>,
    pub(crate) git_status_format: Option<String>,
    pub(crate) git_status_separator: Option<String>,
    pub(crate) git_status_item_format: Option<String>,
    pub(crate) path_before: Option<String>,
    pub(crate) path_selected: Option<String>,
    pub(crate) path_after: Option<String>,
}

impl PromptConfig {
    pub(crate) fn read(path: &Path) -> io::Result<Self> {
        let text = fs::read_to_string(path)?;
        let doc = parse(&text)?;
        Ok(Self {
            cursor_animation: read_bool(&doc, &["editor", "cursor_animation"])?,
            syntax_highlighting: read_bool(&doc, &["editor", "syntax_highlighting"])?,
            completion_enabled: read_bool(&doc, &["editor", "completion"])?,
            left: read_string(&doc, &["prompt", "left"])?,
            right: read_string(&doc, &["prompt", "right"])?,
            transient: read_string(&doc, &["prompt", "transient"])?,
            secondary: read_string(&doc, &["prompt", "secondary"])?,
            clock_format: read_string(&doc, &["widgets", "clock", "format"])?,
            git_status_format: read_string(&doc, &["widgets", "git_status", "format"])?,
            git_status_separator: read_string(&doc, &["widgets", "git_status", "separator"])?,
            git_status_item_format: read_string(&doc, &["widgets", "git_status", "item_format"])?,
            path_before: read_string(&doc, &["widgets", "path", "before"])?,
            path_selected: read_string(&doc, &["widgets", "path", "selected"])?,
            path_after: read_string(&doc, &["widgets", "path", "after"])?,
        })
    }

    pub(crate) fn print(&self) {
        for (name, flag) in [
            ("cursor_animation", self.cursor_animation),
            ("syntax_highlighting", self.syntax_highlighting),
            ("completion_enabled", self.completion_enabled),
        ] {
            if let Some(flag) = flag {
                println!("{name}\t{}", if flag { "31" } else { "30" });
            }
        }
        for (name, value) in [
            ("prompt", &self.left),
            ("rprompt", &self.right),
            ("transient_prompt", &self.transient),
            ("secondary_prompt", &self.secondary),
            ("clock_format", &self.clock_format),
            ("git_status_format", &self.git_status_format),
            ("git_status_separator", &self.git_status_separator),
            ("git_status_item_format", &self.git_status_item_format),
            ("path_before", &self.path_before),
            ("path_selected", &self.path_selected),
            ("path_after", &self.path_after),
        ] {
            if let Some(value) = value {
                let encoded = value
                    .as_bytes()
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>();
                println!(
                    "{name}\t{}",
                    if encoded.is_empty() { "-" } else { &encoded }
                );
            }
        }
    }
}

fn parse(text: &str) -> io::Result<DocumentMut> {
    text.parse::<DocumentMut>()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn read_string(doc: &DocumentMut, keys: &[&str]) -> io::Result<Option<String>> {
    let mut item = doc.as_item();
    for key in keys {
        if item.is_none() {
            return Ok(None);
        }
        item = item
            .as_table_like()
            .ok_or_else(|| invalid(keys))?
            .get(key)
            .unwrap_or(&Item::None);
    }
    if item.is_none() {
        return Ok(None);
    }
    let value = item.as_str().ok_or_else(|| invalid(keys))?;
    if value.contains('\0') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} cannot contain a NUL character", keys.join(".")),
        ));
    }
    Ok(Some(value.to_string()))
}

fn invalid(keys: &[&str]) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("{} must be a string", keys.join(".")),
    )
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
