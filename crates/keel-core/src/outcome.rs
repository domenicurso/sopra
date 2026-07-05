#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeOutcome {
    Accepted(String),
    Cancelled,
}
