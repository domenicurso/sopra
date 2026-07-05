use crate::CommandHandoff;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeOutcome {
    Accepted(CommandHandoff),
    Cancelled,
}
