use serde::{Deserialize, Serialize};

use crate::{TerminalPoint, TerminalSize};

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
    pub alpha: u8,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellStyle {
    Plain,
    Prompt,
    Muted,
    Accent,
    StatusOk,
    StatusError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasCell {
    pub symbol: String,
    pub style: CellStyle,
}

impl Default for CanvasCell {
    fn default() -> Self {
        Self {
            symbol: " ".to_string(),
            style: CellStyle::Plain,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasRow {
    pub cells: Vec<CanvasCell>,
}

impl CanvasRow {
    pub fn blank(columns: u16) -> Self {
        Self {
            cells: vec![CanvasCell::default(); columns as usize],
        }
    }

    pub fn is_blank(&self) -> bool {
        self.cells.iter().all(|cell| cell.symbol == " ")
    }

    pub fn plain_text(&self) -> String {
        self.cells
            .iter()
            .map(|cell| cell.symbol.as_str())
            .collect::<String>()
            .trim_end_matches(' ')
            .to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasBuffer {
    pub size: TerminalSize,
    pub rows: Vec<CanvasRow>,
    pub cursor: Option<VisualCursor>,
}

impl CanvasBuffer {
    pub fn blank(size: TerminalSize) -> Self {
        let rows = (0..size.rows)
            .map(|_| CanvasRow::blank(size.columns))
            .collect::<Vec<_>>();

        Self {
            size,
            rows,
            cursor: None,
        }
    }

    pub fn content_height(&self) -> u16 {
        let last_content_row = self
            .rows
            .iter()
            .rposition(|row| !row.is_blank())
            .map(|row| row as u16 + 1)
            .unwrap_or(0);
        let cursor_height = self
            .cursor
            .filter(|cursor| cursor.visible)
            .map(|cursor| cursor.row.saturating_add(1))
            .unwrap_or(0);

        last_content_row.max(cursor_height)
    }

    pub fn plain_text(&self) -> String {
        self.rows
            .iter()
            .take(self.content_height() as usize)
            .map(CanvasRow::plain_text)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneCursor {
    pub style: CursorStyle,
    pub alpha: u8,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontendScene {
    pub terminal_size: TerminalSize,
    pub prompt_left: String,
    pub prompt_right: String,
    pub editor: EditorSnapshot,
    pub cursor: Option<SceneCursor>,
    pub overlay_anchor: Option<TerminalPoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalOwnershipState {
    Frontend,
    Suspended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalRelease {
    SuspendFrontend,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandHandoff {
    pub command: String,
    pub terminal_release: TerminalRelease,
}
