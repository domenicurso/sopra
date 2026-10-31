use super::Config;
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use toml_edit::DocumentMut;

pub(crate) fn path() -> io::Result<PathBuf> {
    let root = sopra_shell_context::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            sopra_shell_context::var_os("HOME").map(|home| PathBuf::from(home).join(".config"))
        })
        .ok_or_else(|| io::Error::other("HOME and XDG_CONFIG_HOME are unset"))?;
    std::path::absolute(root.join("sopra/config.toml"))
}

impl Config {
    pub(crate) fn read() -> io::Result<Self> {
        Self::read_from(&path()?)
    }

    pub(crate) fn read_from(path: &Path) -> io::Result<Self> {
        let source = match fs::read_to_string(path) {
            Ok(source) => source,
            Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(error),
        };
        let config = Self::parse(&source, path.parent().map(Path::to_owned))?;
        config.validate_widgets()?;
        Ok(config)
    }

    pub(crate) fn effective(&self) -> String {
        self.document.to_string()
    }

    pub(crate) fn validate_widgets(&self) -> io::Result<()> {
        let catalog = crate::prompt::widgets::catalog::read(&self.widgets)?;
        for source in [&self.left, &self.right, &self.transient, &self.secondary] {
            let template = crate::prompt::template::Template::compile(source)?;
            for name in template.references() {
                if !catalog.iter().any(|widget| widget.name == name) {
                    return Err(io::Error::other(format!("unknown prompt widget: {name}")));
                }
            }
        }
        Ok(())
    }
}

pub(super) fn merge(mut defaults: DocumentMut, user: &DocumentMut) -> DocumentMut {
    for (section, item) in user.iter() {
        if section == "widgets" {
            defaults[section] = item.clone();
        } else if let Some(table) = item.as_table_like() {
            for (key, value) in table.iter() {
                defaults[section][key] = value.clone();
            }
        }
    }
    defaults
}
