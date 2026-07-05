#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditResult {
    Continue,
    Submit(String),
    Cancel,
}
