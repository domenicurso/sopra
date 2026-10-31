use super::{Widgets, catalog, settings::Settings};
use std::io;

pub(crate) fn list(settings: &Settings, widgets: Option<&Widgets>) -> io::Result<String> {
    let mut output = String::from("WIDGET\tSOURCE\tSTATUS\n");
    let mut definitions = catalog::read(settings)?;
    if let Some(widgets) = widgets {
        for entry in &widgets.widgets {
            if !definitions
                .iter()
                .any(|definition| definition.name == entry.name)
            {
                definitions.push(catalog::Definition {
                    name: entry.name.clone(),
                    source: entry.source.clone(),
                });
            }
        }
    }
    for definition in definitions {
        let loaded = widgets.and_then(|widgets| {
            widgets
                .widgets
                .iter()
                .find(|entry| entry.name == definition.name)
        });
        let status = loaded.map_or_else(
            || "available".into(),
            |entry| {
                entry
                    .runtime
                    .as_ref()
                    .map_or_else(|| "loaded".into(), |runtime| runtime.status())
            },
        );
        let source = definition
            .source
            .map_or_else(|| "built-in".into(), |path| path.display().to_string());
        output.push_str(&format!(
            "{}\t{}\t{}\n",
            definition.name,
            source.escape_default(),
            status.escape_default()
        ));
    }
    Ok(output)
}
