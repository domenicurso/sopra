use super::settings::Definition;
use sopra::{Action, Context, Event, Text, Widget};
use std::path::Path;

pub(super) struct Entry {
    pub(super) widget: super::background::Background,
    pub(super) name: String,
    pub(super) source: Option<std::path::PathBuf>,
    pub(super) runtime: Option<std::sync::Arc<super::custom::runtime::Runtime>>,
}

impl Entry {
    pub(super) fn new(
        name: &str,
        widget: Box<dyn Widget + Send>,
        cwd: &Path,
        definition: Definition,
    ) -> Self {
        let mut entry = Self {
            widget: super::background::Background::new(widget),
            name: name.into(),
            source: None,
            runtime: None,
        };
        entry.update(cwd, definition);
        entry
    }
    pub(super) fn render(&mut self) -> Text {
        self.widget.display()
    }
    pub(super) fn update(&mut self, cwd: &Path, definition: Definition) {
        self.widget.update(context(cwd, definition.options));
    }
    pub(super) fn handle(&mut self, event: Event) -> Action {
        self.widget.handle(event);
        Action::Consumed
    }
}

pub(super) fn context(cwd: &Path, options: std::collections::BTreeMap<String, String>) -> Context {
    Context {
        cwd: cwd.into(),
        options,
        environment: sopra_shell_context::vars_os()
            .into_iter()
            .map(|(name, value)| {
                (
                    name.to_string_lossy().into_owned(),
                    value.to_string_lossy().into_owned(),
                )
            })
            .collect(),
    }
}
