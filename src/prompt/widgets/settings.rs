use std::{collections::BTreeMap, fs, io, path::PathBuf};
use toml_edit::{DocumentMut, Item};

#[derive(Default)]
pub(super) struct Settings {
    pub(super) root: Option<PathBuf>,
    widgets: BTreeMap<String, BTreeMap<String, String>>,
}

impl Settings {
    pub(super) fn read() -> io::Result<Self> {
        let root = sopra_shell_context::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                sopra_shell_context::var_os("HOME").map(|home| PathBuf::from(home).join(".config"))
            })
            .map(|root| root.join("sopra"));
        let Some(directory) = &root else {
            return Ok(Self::default());
        };
        let mut settings = match fs::read_to_string(directory.join("config.toml")) {
            Ok(text) => parse(&text)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Self::default(),
            Err(error) => return Err(error),
        };
        settings.root = root;
        Ok(settings)
    }

    pub(super) fn options(&self, name: &str) -> BTreeMap<String, String> {
        self.widgets.get(name).cloned().unwrap_or_default()
    }
}

fn parse(text: &str) -> io::Result<Settings> {
    let doc = text.parse::<DocumentMut>().map_err(io::Error::other)?;
    let mut settings = Settings::default();
    let Some(widgets) = doc.get("widgets") else {
        return Ok(settings);
    };
    for (name, item) in table(widgets)?.iter() {
        let mut options = BTreeMap::new();
        for (key, value) in table(item)?.iter() {
            let value = value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string());
            if value.contains('\0') {
                return Err(io::Error::other("widget options cannot contain NUL"));
            }
            options.insert(key.to_owned(), value);
        }
        settings.widgets.insert(name.to_owned(), options);
    }
    Ok(settings)
}

fn table(item: &Item) -> io::Result<&dyn toml_edit::TableLike> {
    item.as_table_like()
        .ok_or_else(|| io::Error::other("widget settings must be TOML tables"))
}

#[cfg(test)]
mod tests {
    use super::parse;
    #[test]
    fn arbitrary_widget_options_do_not_require_registration() -> std::io::Result<()> {
        let settings =
            parse("[widgets.any_name]\nlabel='[dim]hello[/]'\ninterval=12\nenabled=true\n")?;
        let options = settings.options("any_name");
        assert_eq!(
            options.get("label").map(String::as_str),
            Some("[dim]hello[/]")
        );
        assert_eq!(options.get("interval").map(String::as_str), Some("12"));
        assert_eq!(options.get("enabled").map(String::as_str), Some("true"));
        assert!(settings.options("missing").is_empty());
        assert!(parse("widgets=1").is_err());
        assert!(parse("[widgets]\nbroken=1").is_err());
        assert!(parse("[widgets.test]\nvalue=\"\\u0000\"").is_err());
        Ok(())
    }
}
