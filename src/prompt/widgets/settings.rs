use std::{collections::BTreeMap, io, path::PathBuf};
use toml_edit::DocumentMut;

#[derive(Clone, Default)]
pub(crate) struct Definition {
    pub options: BTreeMap<String, String>,
}

#[derive(Clone, Default)]
pub(crate) struct Settings {
    pub root: Option<PathBuf>,
    widgets: BTreeMap<String, Definition>,
}

impl Settings {
    pub(crate) fn parse(doc: &DocumentMut, root: Option<PathBuf>) -> io::Result<Self> {
        let mut settings = Self {
            root,
            ..Self::default()
        };
        let Some(widgets) = doc.get("widgets") else {
            return Ok(settings);
        };
        let widgets = widgets
            .as_table_like()
            .ok_or_else(|| io::Error::other("widgets must be a table"))?;
        for (name, item) in widgets.iter() {
            let table = item
                .as_table_like()
                .ok_or_else(|| io::Error::other("widget options must be a table"))?;
            let mut definition = Definition::default();
            for (key, item) in table.iter() {
                let value = item
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| item.to_string());
                if value.contains('\0') {
                    return Err(io::Error::other("widget options cannot contain NUL"));
                }
                definition.options.insert(key.into(), value);
            }
            settings.widgets.insert(name.into(), definition);
        }
        Ok(settings)
    }

    pub(crate) fn definition(&self, name: &str) -> Definition {
        self.widgets.get(name).cloned().unwrap_or_default()
    }
}
