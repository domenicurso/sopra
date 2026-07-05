mod config;
mod completion;
mod frame;
mod history;
mod input;
mod outcome;
mod prompt;
mod provider;
mod shell;
mod terminal;
mod timing;

pub use completion::{CompletionGroup, CompletionItem, CompletionKind, CompletionRequest, CompletionResponse};
pub use config::SessionConfig;
pub use frame::{
    CursorStyle, EditorSnapshot, SelectionRange, SpanStyle, StyledSpan, SurfaceFrame, SurfaceLine,
    VisualCursor,
};
pub use history::HistoryEntry;
pub use input::{InputEvent, Key};
pub use outcome::RuntimeOutcome;
pub use prompt::{PromptConfig, PromptToken};
pub use provider::{
    ProviderCapability, ProviderCompletionPayload, ProviderHighlightPayload, ProviderOverlayPayload,
    ProviderPromptPayload, ProviderRequest, ProviderResponse, ProviderSuggestionPayload,
};
pub use shell::ShellSnapshot;
pub use terminal::{TerminalPoint, TerminalSize};
pub use timing::RenderConfig;
