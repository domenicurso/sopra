use keel_ui::Size;
use ratatui::{buffer::Buffer, layout::Rect};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderContext {
    pub terminal_columns: u16,
    pub terminal_rows: u16,
    pub max_height: u16,
    pub origin_column: u16,
    pub cursor_row: u16,
    pub row_offset: i16,
    pub scroll_rows: u16,
    pub force_full: bool,
}

impl RenderContext {
    pub const fn new(terminal_columns: u16, max_height: u16, origin_column: u16) -> Self {
        Self {
            terminal_columns: if terminal_columns == 0 {
                1
            } else {
                terminal_columns
            },
            terminal_rows: if max_height == 0 { 1 } else { max_height },
            max_height: if max_height == 0 { 1 } else { max_height },
            origin_column,
            cursor_row: 0,
            row_offset: 1,
            scroll_rows: 0,
            force_full: false,
        }
    }

    pub const fn terminal_rows(mut self, terminal_rows: u16) -> Self {
        self.terminal_rows = if terminal_rows == 0 { 1 } else { terminal_rows };
        self
    }

    pub const fn cursor_row(mut self, cursor_row: u16) -> Self {
        self.cursor_row = cursor_row;
        self
    }

    pub const fn row_offset(mut self, row_offset: i16) -> Self {
        self.row_offset = row_offset;
        self
    }

    pub const fn scroll_rows(mut self, scroll_rows: u16) -> Self {
        self.scroll_rows = scroll_rows;
        self
    }

    pub const fn full_repaint(mut self, force_full: bool) -> Self {
        self.force_full = force_full;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedFrame {
    pub area: Rect,
    pub used_size: Size,
    pub buffer: Buffer,
    pub terminal_columns: u16,
    pub terminal_rows: u16,
    pub origin_column: u16,
    pub cursor_row: u16,
    pub anchor_row: u16,
    pub row_offset: i16,
    pub scroll_rows: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameDiff {
    pub changed_cells: usize,
    pub changed_rows: Vec<u16>,
    pub cleared_rows: Vec<u16>,
    pub previous_area: Rect,
    pub next_area: Rect,
    pub previous_terminal_columns: u16,
    pub next_terminal_columns: u16,
    pub previous_terminal_rows: u16,
    pub next_terminal_rows: u16,
    pub previous_origin: u16,
    pub next_origin: u16,
    pub previous_cursor_row: u16,
    pub next_cursor_row: u16,
    pub previous_anchor_row: u16,
    pub next_anchor_row: u16,
    pub previous_row_offset: i16,
    pub next_row_offset: i16,
    pub previous_scroll_rows: u16,
    pub next_scroll_rows: u16,
    pub terminal_changed: bool,
}

impl FrameDiff {
    pub fn changed(&self) -> bool {
        self.changed_cells > 0
            || !self.cleared_rows.is_empty()
            || self.previous_area != self.next_area
            || self.terminal_changed
            || self.previous_scroll_rows != self.next_scroll_rows
            || self.origin_changed()
    }

    pub fn origin_changed(&self) -> bool {
        self.terminal_changed
            || self.previous_scroll_rows != self.next_scroll_rows
            || self.previous_origin != self.next_origin
            || self.previous_anchor_row != self.next_anchor_row
            || (self.previous_cursor_row == self.next_cursor_row
                && self.previous_row_offset != self.next_row_offset)
    }

    pub fn scroll_rows_added(&self) -> u16 {
        self.next_scroll_rows
            .saturating_sub(self.previous_scroll_rows)
    }

    pub fn scroll_rows_removed(&self) -> u16 {
        self.previous_scroll_rows
            .saturating_sub(self.next_scroll_rows)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchOp {
    SaveCursor,
    HideCursor,
    ScrollUp {
        rows: u16,
    },
    ScrollDown {
        rows: u16,
    },
    MoveToSurface,
    ClearSurface {
        origin_column: u16,
        row_offset: i16,
        width: u16,
        height: u16,
    },
    ClearSpan {
        row: u16,
        width: u16,
    },
    PaintRow {
        row: u16,
    },
    RestoreCursor,
    ShowCursor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderTransaction {
    pub origin_column: u16,
    pub row_offset: i16,
    pub width: u16,
    pub height: u16,
    pub ops: Vec<PatchOp>,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedRegion {
    pub frame: RenderedFrame,
    pub diff: FrameDiff,
    pub transaction: RenderTransaction,
    pub used_rows: u16,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RenderError {
    #[error("rendered surface has no columns")]
    EmptyWidth,
}
