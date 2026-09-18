use std::{ops::Range, path::PathBuf, time::Duration};

#[derive(Debug, Clone)]
pub(crate) struct Request {
    pub(crate) line: String,
    pub(crate) cursor: usize,
    pub(crate) context_line: String,
    pub(crate) context_cursor: usize,
    pub(crate) replace: Range<usize>,
    pub(crate) cwd: PathBuf,
    pub(crate) generation: u64,
    pub(crate) context_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompletionResponseSource {
    Local,
    Zshrs,
}

#[derive(Debug, Clone)]
pub(crate) struct CompletionResponse {
    pub(crate) line: String,
    pub(crate) cursor: usize,
    pub(crate) context_line: String,
    pub(crate) context_cursor: usize,
    pub(crate) context_key: String,
    pub(crate) items: Vec<CompletionItem>,
    pub(crate) generation: u64,
    pub(crate) source: CompletionResponseSource,
    pub(crate) incomplete: bool,
    pub(crate) elapsed: Duration,
}

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
    Variable,
    Array,
    File,
    Directory,
    Host,
    User,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompletionSource {
    Zshrs,
    CommandIndex,
    ShellContext,
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
    pub(crate) suffix: String,
    pub(crate) cursor_offset: Option<usize>,
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
            suffix: String::new(),
            cursor_offset: None,
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
            suffix: String::new(),
            cursor_offset: None,
            score: 0.0,
            source,
            match_indices: Vec::new(),
        }
    }

    pub(crate) fn with_suffix(mut self, suffix: impl Into<String>, cursor_offset: usize) -> Self {
        self.suffix = suffix.into();
        self.cursor_offset = Some(cursor_offset);
        self
    }
}

pub(crate) type CompletionItem = Completion;

pub(crate) fn normalize_description(display: &str, description: Option<String>) -> Option<String> {
    let description = description?
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if description.is_empty() {
        return None;
    }
    let remainder = description.strip_prefix(display).unwrap_or(&description);
    let remainder = remainder
        .strip_prefix(':')
        .unwrap_or(remainder)
        .trim_start();
    let remainder = remainder
        .strip_prefix("--")
        .unwrap_or(remainder)
        .trim_start();
    (!remainder.is_empty()).then(|| remainder.to_string())
}

#[cfg(test)]
fn nonempty(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}
