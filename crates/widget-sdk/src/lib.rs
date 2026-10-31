mod events;
#[doc(hidden)]
pub mod protocol;
mod runner;
mod text;

pub use self::{
    events::{Event, Key},
    runner::run,
    text::{Emphasis, Part, Text},
};
use std::{collections::BTreeMap, path::PathBuf, time::Duration};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Context {
    pub cwd: PathBuf,
    pub options: BTreeMap<String, String>,
    pub environment: BTreeMap<String, String>,
}

impl Context {
    pub fn option<'a>(&'a self, name: &str, default: &'a str) -> &'a str {
        self.options.get(name).map_or(default, String::as_str)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Action {
    #[default]
    Ignored,
    Consumed,
    ChangeDirectory(PathBuf),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Refresh {
    #[default]
    OnContext,
    Every(Duration),
}

pub trait Widget {
    fn render(&mut self, context: &Context) -> Text;
    fn handle(&mut self, _event: Event, _context: &Context) -> Action {
        Action::Ignored
    }
    fn interactive(&self) -> bool {
        false
    }
    fn active(&self) -> bool {
        false
    }
    fn refresh(&self) -> Refresh {
        Refresh::OnContext
    }
}
