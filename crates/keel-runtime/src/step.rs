#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeStep {
    Continue { redraw: bool },
    Accepted(String),
    Cancelled,
}
