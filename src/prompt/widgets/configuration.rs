use super::{Focus, Widgets, builtins, catalog, custom, entry::Entry, settings::Settings};
use crate::prompt::template::Template;
use std::{
    io,
    path::{Path, PathBuf},
};

impl Widgets {
    pub(crate) fn configured(
        left: String,
        right: String,
        cwd: PathBuf,
        settings: &Settings,
    ) -> io::Result<Self> {
        let mut widgets = Self {
            prompt_template: Template::compile(&left)?,
            right_template: Template::compile(&right)?,
            widgets: Vec::new(),
            focused: Focus::Idle,
        };
        widgets.reconfigure(&left, &right, &cwd, settings)?;
        Ok(widgets)
    }

    pub(crate) fn reconfigure(
        &mut self,
        left: &str,
        right: &str,
        cwd: &Path,
        settings: &Settings,
    ) -> io::Result<()> {
        let left = Template::compile(left)?;
        let right = Template::compile(right)?;
        let catalog = catalog::read(settings)?;
        for name in left.references().chain(right.references()) {
            if !catalog.iter().any(|entry| entry.name == name) {
                return Err(io::Error::other(format!("unknown prompt widget: {name}")));
            }
        }
        let mut retained = std::mem::take(&mut self.widgets);
        for definition in catalog
            .into_iter()
            .filter(|entry| super::referenced(&left, &right, &entry.name))
        {
            let entry = if let Some(index) = retained.iter().position(|entry| {
                entry.name == definition.name && entry.source == definition.source
            }) {
                retained.remove(index)
            } else {
                create(&definition, cwd, settings)?
            };
            self.widgets.push(entry);
        }
        self.prompt_template = left;
        self.right_template = right;
        self.focused = Focus::Idle;
        self.update(cwd, settings);
        Ok(())
    }
}

fn create(definition: &catalog::Definition, cwd: &Path, settings: &Settings) -> io::Result<Entry> {
    if let Some(path) = &definition.source {
        return Ok(custom::load(path.clone(), &definition.name, settings, cwd));
    }
    let factory = builtins::REGISTRY
        .iter()
        .find(|(name, _)| *name == definition.name)
        .map(|(_, create)| create)
        .ok_or_else(|| io::Error::other("missing built-in widget factory"))?;
    Ok(Entry::new(
        &definition.name,
        factory(),
        cwd,
        settings.definition(&definition.name),
    ))
}
