mod compiler;
mod process;
mod runtime;
mod transport;
// This file is also embedded as the standalone public SDK compiled with user widgets.
#[allow(dead_code)]
mod sdk;

use super::{Widget, WidgetAction};
use crate::input::Key;
use runtime::Runtime;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};

static RUNTIMES: OnceLock<Mutex<BTreeMap<PathBuf, Arc<Runtime>>>> = OnceLock::new();

struct Custom {
    token: String,
    format: String,
    runtime: Arc<Runtime>,
}

pub(super) fn discover(
    templates: (&str, &str),
    cwd: &Path,
    settings: &super::settings::Settings,
) -> Vec<Box<dyn Widget>> {
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
    settings: &super::settings::Settings,
    templates: (&str, &str),
    cwd: &Path,
) -> Option<Box<dyn Widget>> {
    let name = path.file_stem()?.to_str()?;
    if path.extension()? != "rs"
        || !valid_name(name)
        || !super::referenced(templates.0, templates.1, name)
    {
        return None;
    }
    if super::builtins::REGISTRY
        .iter()
        .any(|(builtin, _)| *builtin == name)
    {
        return None;
    }
    let token = format!("{{{name}}}");
    let options = settings.options(name);
    let format = options
        .get("format")
        .cloned()
        .unwrap_or_else(|| "{text}".into());
    let context = context(cwd, &options);
    let mut runtimes = RUNTIMES
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let runtime = runtimes
        .entry(path.clone())
        .or_insert_with(|| Arc::new(Runtime::start(path, context.clone())));
    runtime.update(context);
    Some(Box::new(Custom {
        token,
        format,
        runtime: Arc::clone(runtime),
    }))
}

impl Widget for Custom {
    fn token(&self) -> &str {
        &self.token
    }
    fn markup(&self) -> bool {
        true
    }
    fn render(&self) -> String {
        let text = self.runtime.display().0;
        if text.is_empty() {
            return text;
        }
        self.format.replace("{text}", &text.replace('[', "[["))
    }
    fn interactive(&self) -> bool {
        self.runtime.display().2
    }
    fn active(&self) -> bool {
        self.runtime.display().1
    }
    fn handle(&mut self, key: Key) -> WidgetAction {
        let (_, active, interactive) = self.runtime.display();
        if !active && !(interactive && key == Key::PromptWidgets) {
            return WidgetAction::Ignored;
        }
        match self.runtime.key(format!("{key:?}")) {
            Some(path) => WidgetAction::ChangeDirectory(path),
            None => WidgetAction::Consumed,
        }
    }
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn context(cwd: &Path, options: &BTreeMap<String, String>) -> String {
    let environment = sopra_shell_context::vars_os()
        .into_iter()
        .map(|(name, value)| {
            (
                name.to_string_lossy().into_owned(),
                value.to_string_lossy().into_owned(),
            )
        })
        .collect();
    format!(
        "W1\t{}\t{}\t{}",
        sdk::encode(&cwd.to_string_lossy()),
        sdk::encode(&pairs(options)),
        sdk::encode(&pairs(&environment))
    )
}

fn pairs(values: &BTreeMap<String, String>) -> String {
    values
        .iter()
        .map(|(key, value)| format!("{key}\0{value}\0"))
        .collect()
}
