use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptConfig {
    pub active_left: Vec<PromptToken>,
    pub active_right: Vec<PromptToken>,
    pub transient_left: Vec<PromptToken>,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            active_left: vec![
                PromptToken::Literal("keel".to_string()),
                PromptToken::Literal("> ".to_string()),
            ],
            active_right: Vec::new(),
            transient_left: vec![PromptToken::Literal("> ".to_string())],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromptToken {
    Literal(String),
    CurrentDirectory,
    ExitStatus,
    CommandDuration,
    Widget(String),
}
