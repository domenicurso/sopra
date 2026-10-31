mod compiler;
mod process;
mod runtime;
mod session;
mod transport;
mod watch;

use super::{
    entry::{self, Entry},
    settings::Settings,
};
use runtime::Runtime;
use sopra::{Action, Context, Event, Refresh, Text, Widget};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

struct Remote {
    runtime: Arc<Runtime>,
}

pub(super) fn discover(
    templates: (
        &crate::prompt::template::Template,
        &crate::prompt::template::Template,
    ),
    cwd: &Path,
    settings: &Settings,
) -> Vec<Entry> {
    let Some(root) = settings.root.as_ref() else {
        return Vec::new();
    };
    let directory = root.join("widgets");
    let files = match std::fs::read_dir(&directory) {
        Ok(files) => files,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(error) => {
            eprintln!("sopra: {}: {error}", directory.display());
            return Vec::new();
        }
    };
    let mut paths = files
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect::<Vec<_>>();
    paths.sort();
    paths
        .into_iter()
        .filter_map(|path| load(path, settings, templates, cwd))
        .collect()
}

fn load(
    path: PathBuf,
    settings: &Settings,
    templates: (
        &crate::prompt::template::Template,
        &crate::prompt::template::Template,
    ),
    cwd: &Path,
) -> Option<Entry> {
    let name = path.file_stem()?.to_str()?.to_owned();
    if path.extension()? != "rs"
        || !valid_name(&name)
        || !super::referenced(templates.0, templates.1, &name)
    {
        return None;
    }
    if super::builtins::REGISTRY
        .iter()
        .any(|(builtin, _)| *builtin == name)
    {
        return None;
    }
    let definition = settings.definition(&name);
    let context = entry::context(cwd, definition.options.clone());
    let runtime = Arc::new(Runtime::start(path, context));
    Some(Entry::new(
        &name,
        Box::new(Remote { runtime }),
        cwd,
        definition,
    ))
}

impl Widget for Remote {
    fn render(&mut self, context: &Context) -> Text {
        self.runtime.update(context.clone());
        self.runtime.display().text
    }
    fn interactive(&self) -> bool {
        self.runtime.display().interactive
    }
    fn active(&self) -> bool {
        self.runtime.display().active
    }
    fn refresh(&self) -> Refresh {
        Refresh::Every(Duration::from_millis(16))
    }
    fn handle(&mut self, event: Event, _context: &Context) -> Action {
        self.runtime.key(event)
    }
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}
