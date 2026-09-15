#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum CompletionKind {
    #[default]
    Generic,
    File,
    Directory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompletionItem {
    pub(crate) label: String,
    pub(crate) detail: String,
    pub(crate) replacement: String,
    pub(crate) kind: CompletionKind,
    pub(crate) match_indices: Vec<usize>,
}

impl CompletionItem {
    pub(crate) fn new(
        label: impl Into<String>,
        detail: impl Into<String>,
        replacement: impl Into<String>,
        kind: CompletionKind,
    ) -> Self {
        Self {
            label: label.into(),
            detail: detail.into(),
            replacement: replacement.into(),
            kind,
            match_indices: Vec::new(),
        }
    }
}
