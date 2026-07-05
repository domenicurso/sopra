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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromptSpanStyle {
    Plain,
    Prompt,
    Muted,
    Accent,
    StatusOk,
    StatusError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptSpan {
    pub text: String,
    pub style: PromptSpanStyle,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PromptSurface {
    pub spans: Vec<PromptSpan>,
}

impl PromptSurface {
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    pub fn plain_text(&self) -> String {
        self.spans
            .iter()
            .map(|span| span.text.as_str())
            .collect::<String>()
    }

    pub fn push(&mut self, text: impl Into<String>, style: PromptSpanStyle) {
        let text = text.into();
        if text.is_empty() {
            return;
        }

        if let Some(previous) = self.spans.last_mut() {
            if previous.style == style {
                previous.text.push_str(&text);
                return;
            }
        }

        self.spans.push(PromptSpan { text, style });
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
