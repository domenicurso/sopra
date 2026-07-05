#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontendMode {
    PromptEditing,
    CommandSubmission,
    CommandExecutionHandoff,
    Suspended,
    Cancelled,
    Recovery,
}
