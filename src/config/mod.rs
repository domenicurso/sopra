use crate::prompt::{template::Template, widgets::settings::Settings};
mod file;
pub(crate) use file::path;
use std::{io, path::PathBuf};
use toml_edit::{DocumentMut, Item};

#[derive(Clone)]
pub(crate) struct Config {
    document: DocumentMut,
    pub(crate) cursor_animation: bool,
    pub(crate) syntax_highlighting: bool,
    pub(crate) completion: bool,
    pub(crate) autosuggestions: bool,
    pub(crate) autosuggestion_format: String,
    pub(crate) autosuggestion_metadata_format: String,
    pub(crate) left: String,
    pub(crate) right: String,
    pub(crate) transient: String,
    pub(crate) secondary: String,
    pub(crate) widgets: Settings,
}

impl Config {
    fn parse(source: &str, root: Option<PathBuf>) -> io::Result<Self> {
        let user = source.parse::<DocumentMut>().map_err(io::Error::other)?;
        let defaults = include_str!("defaults.toml")
            .parse::<DocumentMut>()
            .map_err(io::Error::other)?;
        validate(&user, &defaults)?;
        let value =
            |section, name| field(&user, section, name).or_else(|| field(&defaults, section, name));
        let string = |section, name| string(value(section, name), section, name);
        let boolean = |section, name| boolean(value(section, name), section, name);
        let document = file::merge(defaults.clone(), &user);
        let config = Self {
            document,
            cursor_animation: boolean("editor", "cursor_animation")?,
            syntax_highlighting: boolean("editor", "syntax_highlighting")?,
            autosuggestions: boolean("editor", "autosuggestions")?,
            completion: boolean("completion", "enabled")?,
            autosuggestion_format: string("editor", "autosuggestion_format")?,
            autosuggestion_metadata_format: string("editor", "autosuggestion_metadata_format")?,
            left: string("prompt", "left")?,
            right: string("prompt", "right")?,
            transient: string("prompt", "transient")?,
            secondary: string("prompt", "secondary")?,
            widgets: Settings::parse(&user, root)?,
        };
        for source in [
            &config.left,
            &config.right,
            &config.transient,
            &config.secondary,
            &config.autosuggestion_format,
            &config.autosuggestion_metadata_format,
        ] {
            Template::compile(source)?;
        }
        Ok(config)
    }
}

fn validate(user: &DocumentMut, defaults: &DocumentMut) -> io::Result<()> {
    for (section, item) in user.iter() {
        if section == "widgets" {
            continue;
        }
        let known = defaults
            .get(section)
            .and_then(Item::as_table_like)
            .ok_or_else(|| io::Error::other(format!("unknown configuration section: {section}")))?;
        let table = item
            .as_table_like()
            .ok_or_else(|| invalid(section, "", "table"))?;
        for (name, _) in table.iter() {
            if known.get(name).is_none() {
                return Err(io::Error::other(format!(
                    "unknown configuration option: {section}.{name}"
                )));
            }
        }
    }
    Ok(())
}

fn field<'a>(doc: &'a DocumentMut, section: &str, name: &str) -> Option<&'a Item> {
    doc.get(section)?.as_table_like()?.get(name)
}
fn string(value: Option<&Item>, section: &str, name: &str) -> io::Result<String> {
    let text = value
        .and_then(Item::as_str)
        .ok_or_else(|| invalid(section, name, "string"))?;
    if text.contains('\0') {
        return Err(invalid(section, name, "string without NUL"));
    }
    Ok(text.into())
}
fn boolean(value: Option<&Item>, section: &str, name: &str) -> io::Result<bool> {
    value
        .and_then(Item::as_bool)
        .ok_or_else(|| invalid(section, name, "boolean"))
}
fn invalid(section: &str, name: &str, kind: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("{section}.{name} must be a {kind}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_and_user_options_share_one_validated_config() -> io::Result<()> {
        let config = Config::parse(
            "[prompt]\nleft='[blue]$path[/blue]'\n[completion]\nenabled=false\n[widgets.clock]\nformat='%H'",
            None,
        )?;
        assert!(!config.completion);
        assert!(config.cursor_animation);
        assert_eq!(config.widgets.definition("clock").options["format"], "%H");
        for invalid in [
            "prompt=42",
            "[editor]\ncompletion=false",
            "[unknown]\nkey=true",
            "[prompt]\nleft=42",
            "[prompt]\nleft='[red]x[/blue]'",
            "[editor]\nautosuggestions='yes'",
        ] {
            assert!(Config::parse(invalid, None).is_err());
        }
        Ok(())
    }
}
