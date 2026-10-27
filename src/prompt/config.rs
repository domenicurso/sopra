use std::{fs, io, path::Path};

use toml_edit::{DocumentMut, Item, value};

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct PromptConfig {
    pub(crate) left: Option<String>,
    pub(crate) right: Option<String>,
    pub(crate) transient: Option<String>,
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
            left: read_string(&doc, &["prompt", "left"])?,
            right: read_string(&doc, &["prompt", "right"])?,
            transient: read_string(&doc, &["prompt", "transient"])?,
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
        for (name, value) in [
            ("prompt", &self.left),
            ("rprompt", &self.right),
            ("transient_prompt", &self.transient),
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

pub(crate) fn update(path: &Path, config: &PromptConfig) -> io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let mut doc = parse(&fs::read_to_string(path)?)?;
    set(&mut doc, &["prompt", "left"], &config.left)?;
    set(&mut doc, &["prompt", "right"], &config.right)?;
    set(&mut doc, &["prompt", "transient"], &config.transient)?;
    set(
        &mut doc,
        &["widgets", "clock", "format"],
        &config.clock_format,
    )?;
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&temporary, doc.to_string())?;
    fs::rename(temporary, path)
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

fn set(doc: &mut DocumentMut, keys: &[&str], text: &Option<String>) -> io::Result<()> {
    let Some(text) = text else {
        return Ok(());
    };
    let mut table = doc.as_table_mut();
    for key in &keys[..keys.len() - 1] {
        if !table.contains_key(key) {
            table.insert(key, Item::Table(toml_edit::Table::new()));
        }
        table = table
            .get_mut(key)
            .and_then(Item::as_table_mut)
            .ok_or_else(|| invalid(keys))?;
    }
    let key = keys[keys.len() - 1];
    if let Some(existing) = table.get_mut(key) {
        let previous = existing.as_value().ok_or_else(|| invalid(keys))?;
        let mut replacement = value(text);
        if let Some(next) = replacement.as_value_mut() {
            *next.decor_mut() = previous.decor().clone();
        }
        *existing = replacement;
    } else {
        table.insert(key, value(text));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_and_updates_prompt_templates_without_losing_comments() -> io::Result<()> {
        let file = std::env::temp_dir().join(format!("sopra-prompt-{}.toml", std::process::id()));
        fs::write(
            &file,
            "# keep me\n[prompt]\nright = '[dim]{clock}[/]' # inline comment\n[widgets.clock]\nformat = '%H:%M'\n[widgets.git_status]\nformat = '<{items}>'\nseparator = ' / '\n",
        )?;
        let mut config = PromptConfig::read(&file)?;
        assert_eq!(config.right.as_deref(), Some("[dim]{clock}[/]"));
        assert_eq!(config.git_status_format.as_deref(), Some("<{items}>"));
        assert_eq!(config.git_status_separator.as_deref(), Some(" / "));
        config.right = Some("{git}".into());
        update(&file, &config)?;
        let text = fs::read_to_string(&file)?;
        assert!(text.contains("# keep me") && text.contains("# inline comment"));
        assert_eq!(PromptConfig::read(&file)?, config);
        fs::remove_file(file)
    }
    #[test]
    fn invalid_field_types_report_the_field() -> io::Result<()> {
        let doc = parse("[widgets.clock]\nformat = 42\n")?;
        let result = read_string(&doc, &["widgets", "clock", "format"]);
        assert!(
            result
                .err()
                .is_some_and(|error| error.to_string().contains("widgets.clock.format"))
        );
        let doc = parse("[prompt]\nleft = \"\\u0000\"\n")?;
        assert!(
            read_string(&doc, &["prompt", "left"])
                .err()
                .is_some_and(|error| error.to_string().contains("NUL"))
        );
        Ok(())
    }
}
