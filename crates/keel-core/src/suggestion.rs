#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub label: String,
    pub detail: String,
    pub(crate) replacement: String,
    pub(crate) match_indices: Vec<usize>,
}

impl Suggestion {
    pub fn new(label: impl Into<String>, detail: impl Into<String>) -> Self {
        let label = label.into();
        Self {
            replacement: label.clone(),
            label,
            detail: detail.into(),
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
            replacement: replacement.into(),
            match_indices: Vec::new(),
        }
    }

    pub fn replacement(&self) -> &str {
        &self.replacement
    }

    /// Byte offsets of the characters selected by the fuzzy matcher.
    pub fn match_indices(&self) -> &[usize] {
        &self.match_indices
    }
}

pub trait CompletionProvider {
    fn complete(&self, line: &str, cursor: usize, cwd: &str) -> Vec<Suggestion>;
}
