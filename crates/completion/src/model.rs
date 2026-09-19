use std::{ops::Range, path::PathBuf, time::Duration};

#[derive(Debug, Clone)]
pub struct Request {
    pub line: String,
    pub cursor: usize,
    pub context_line: String,
    pub context_cursor: usize,
    pub replace: Range<usize>,
    pub cwd: PathBuf,
    pub generation: u64,
    pub context_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionResponseSource {
    Local,
}

#[derive(Debug, Clone)]
pub struct CompletionResponse {
    pub line: String,
    pub cursor: usize,
    pub context_line: String,
    pub context_cursor: usize,
    pub context_key: String,
    pub items: Vec<CompletionItem>,
    pub generation: u64,
    pub source: CompletionResponseSource,
    pub incomplete: bool,
    pub elapsed: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompletionKind {
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
pub enum CompletionSource {
    CommandIndex,
    ShellContext,
    Filesystem,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Completion {
    pub insert: String,
    pub display: String,
    pub description: Option<String>,
    pub kind: CompletionKind,
    pub group: Option<String>,
    pub location: Option<PathBuf>,
    pub replace: Range<usize>,
    pub suffix: String,
    pub cursor_offset: Option<usize>,
    pub score: f32,
    pub source: CompletionSource,
    pub match_indices: Vec<usize>,
}

impl Completion {
    pub fn new(
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
            source: CompletionSource::CommandIndex,
            match_indices: Vec::new(),
        }
    }

    pub fn with_range(
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

    pub fn with_suffix(mut self, suffix: impl Into<String>, cursor_offset: usize) -> Self {
        self.suffix = suffix.into();
        self.cursor_offset = Some(cursor_offset);
        self
    }
}

pub type CompletionItem = Completion;

fn nonempty(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}
