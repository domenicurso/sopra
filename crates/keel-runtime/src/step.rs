use keel_core::CommandHandoff;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeStep {
    Continue { redraw: bool },
    Accepted(CommandHandoff),
    Cancelled,
}
