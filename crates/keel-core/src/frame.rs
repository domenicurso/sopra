use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorSnapshot {
    pub buffer: String,
    pub cursor: usize,
    pub selection: Option<SelectionRange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpanStyle {
    Plain,
    Prompt,
    Suggestion,
    Muted,
    Accent,
    StatusOk,
    StatusError,
    CursorBlock { alpha: u8 },
    CursorBeam { alpha: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyledSpan {
    pub text: String,
    pub style: SpanStyle,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SurfaceLine {
    pub spans: Vec<StyledSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CursorStyle {
    Block,
    Beam,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualCursor {
    pub row: u16,
    pub column: u16,
    pub style: CursorStyle,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SurfaceFrame {
    pub lines: Vec<SurfaceLine>,
    pub cursor: Option<VisualCursor>,
}

impl SurfaceFrame {
    pub fn plain_text(&self) -> String {
        self.lines
            .iter()
            .map(|line| line.spans.iter().map(|span| span.text.as_str()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }
}
