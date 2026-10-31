mod compiler;
mod process;
pub(super) mod runtime;
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

pub(super) fn load(path: PathBuf, name: &str, settings: &Settings, cwd: &Path) -> Entry {
    let definition = settings.definition(name);
    let context = entry::context(cwd, definition.options.clone());
    let runtime = Arc::new(Runtime::start(path.clone(), context));
    let mut entry = Entry::new(
        name,
        Box::new(Remote {
            runtime: runtime.clone(),
        }),
        cwd,
        definition,
    );
    entry.source = Some(path);
    entry.runtime = Some(runtime);
    entry
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
