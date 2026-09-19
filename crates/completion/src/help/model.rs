use std::{path::PathBuf, time::Duration};

use super::super::Request;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct HelpSpec {
    pub(crate) commands: Vec<HelpCommand>,
    pub(crate) options: Vec<HelpOption>,
    pub(crate) positionals: Vec<HelpPositional>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HelpCommand {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) aliases: Vec<String>,
    pub(crate) positional: Option<ArgumentKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HelpOption {
    pub(crate) names: Vec<String>,
    pub(crate) description: Option<String>,
    pub(crate) values: Vec<String>,
    pub(crate) expects_value: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HelpPositional {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) kind: ArgumentKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArgumentKind {
    File,
    Directory,
    Url,
    Number,
    Value,
    Text,
}

#[derive(Debug, Clone)]
pub(crate) struct HelpRequest {
    pub(crate) key: String,
    pub(crate) command: Option<PathBuf>,
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) cwd: PathBuf,
    pub(crate) request: Request,
}

#[derive(Debug)]
pub(crate) struct HelpResponse {
    pub(crate) key: String,
    pub(crate) request: Request,
    pub(crate) spec: Option<HelpSpec>,
    pub(crate) elapsed: Duration,
}
