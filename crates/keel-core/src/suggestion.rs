#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub label: String,
    pub detail: String,
    kind: SuggestionKind,
    pub(crate) replacement: String,
    pub(crate) match_indices: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SuggestionKind {
    #[default]
    Generic,
    File,
    Directory,
}

impl Suggestion {
    pub fn new(label: impl Into<String>, detail: impl Into<String>) -> Self {
        let label = label.into();
        Self {
            replacement: label.clone(),
            label,
            detail: detail.into(),
            kind: SuggestionKind::Generic,
            match_indices: Vec::new(),
        }
    }

    pub fn with_replacement(
        label: impl Into<String>,
        detail: impl Into<String>,
        replacement: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            detail: detail.into(),
            kind: SuggestionKind::Generic,
            replacement: replacement.into(),
            match_indices: Vec::new(),
        }
    }

    pub fn with_kind(mut self, kind: SuggestionKind) -> Self {
        self.kind = kind;
        self
    }

    /// The candidate token that replaces the active shell token on accept.
    pub fn replacement(&self) -> &str {
        &self.replacement
    }

    pub fn kind(&self) -> SuggestionKind {
        self.kind
    }

    /// Byte offsets of the characters selected by the fuzzy matcher.
    pub fn match_indices(&self) -> &[usize] {
        &self.match_indices
    }
}

pub trait CompletionProvider {
    fn complete(&self, line: &str, cursor: usize, cwd: &str) -> Vec<Suggestion>;
}
