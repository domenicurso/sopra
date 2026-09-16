use std::{ops::Range, path::PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum CompletionKind {
    #[default]
    Generic,
    Command,
    Alias,
    Function,
    Builtin,
    Option,
    Subcommand,
    Value,
    Positional,
    File,
    Directory,
    Host,
    User,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompletionSource {
    Zshrs,
    CommandIndex,
    Filesystem,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Completion {
    pub(crate) insert: String,
    pub(crate) display: String,
    pub(crate) description: Option<String>,
    pub(crate) kind: CompletionKind,
    pub(crate) group: Option<String>,
    pub(crate) location: Option<PathBuf>,
    pub(crate) replace: Range<usize>,
    pub(crate) score: f32,
    pub(crate) source: CompletionSource,
    pub(crate) match_indices: Vec<usize>,
}

impl Completion {
    #[cfg(test)]
    pub(crate) fn new(
        display: impl Into<String>,
        description: impl Into<String>,
        insert: impl Into<String>,
        kind: CompletionKind,
    ) -> Self {
        Self {
            insert: insert.into(),
            display: display.into(),
            description: nonempty(description.into()),
            kind,
            group: None,
            location: None,
            replace: 0..0,
            score: 0.0,
            source: CompletionSource::Zshrs,
            match_indices: Vec::new(),
        }
    }

    pub(crate) fn with_range(
        display: impl Into<String>,
        insert: impl Into<String>,
        description: Option<String>,
        kind: CompletionKind,
        replace: Range<usize>,
        source: CompletionSource,
    ) -> Self {
        Self {
            insert: insert.into(),
            display: display.into(),
            description,
            kind,
            group: None,
            location: None,
            replace,
            score: 0.0,
            source,
            match_indices: Vec::new(),
        }
    }
}

pub(crate) type CompletionItem = Completion;

#[cfg(test)]
fn nonempty(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}
