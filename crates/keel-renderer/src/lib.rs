use std::fmt::Write as _;

use keel_ui::{Constraints, Scene, Size};
use ratatui::{
    buffer::{Buffer, Cell},
    layout::Rect,
    style::{Color, Modifier, Style},
};
use thiserror::Error;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderContext {
    pub terminal_columns: u16,
    pub max_height: u16,
    pub origin_column: u16,
    pub row_offset: i16,
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
            max_height: if max_height == 0 { 1 } else { max_height },
            origin_column,
            row_offset: 1,
            force_full: false,
        }
    }

    pub const fn row_offset(mut self, row_offset: i16) -> Self {
        self.row_offset = row_offset;
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
    pub origin_column: u16,
    pub row_offset: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameDiff {
    pub changed_cells: usize,
    pub changed_rows: Vec<u16>,
    pub cleared_rows: Vec<u16>,
    pub previous_area: Rect,
    pub next_area: Rect,
    pub previous_origin: u16,
    pub next_origin: u16,
    pub previous_row_offset: i16,
    pub next_row_offset: i16,
}

impl FrameDiff {
    pub fn changed(&self) -> bool {
        self.changed_cells > 0
            || !self.cleared_rows.is_empty()
            || self.previous_area != self.next_area
            || self.previous_origin != self.next_origin
            || self.previous_row_offset != self.next_row_offset
    }

    pub fn origin_changed(&self) -> bool {
        self.previous_origin != self.next_origin || self.previous_row_offset != self.next_row_offset
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchOp {
    SaveCursor,
    HideCursor,
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

#[derive(Debug, Default)]
pub struct Renderer {
    previous: Option<RenderedFrame>,
}

impl Renderer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn render(
        &mut self,
        scene: &Scene,
        context: RenderContext,
    ) -> Result<RenderedRegion, RenderError> {
        let max_width = context
            .terminal_columns
            .saturating_sub(context.origin_column)
            .max(1);
        let measured = scene.measure(Constraints::new(max_width, context.max_height));
        let width = measured.width.min(max_width);
        if width == 0 {
            return Err(RenderError::EmptyWidth);
        }
        let height = measured.height.max(1).min(context.max_height.max(1));
        let origin_column = context
            .origin_column
            .min(context.terminal_columns.saturating_sub(width));
        let area = Rect::new(0, 0, width, height);
        let mut buffer = Buffer::empty(area);
        scene.render(area, &mut buffer);
        let frame = RenderedFrame {
            area,
            used_size: Size::new(width, height),
            buffer,
            origin_column,
            row_offset: context.row_offset,
        };
        let diff = self.diff(Some(&frame));
        let payload = AnsiWriter::paint(&frame, &diff, context.force_full);
        let transaction = RenderTransaction {
            origin_column,
            row_offset: context.row_offset,
            width,
            height,
            ops: patch_ops(&diff, &frame, context.force_full),
            payload,
        };
        let rendered = RenderedRegion {
            frame: frame.clone(),
            diff,
            transaction,
            used_rows: height,
        };
        self.previous = Some(frame);
        Ok(rendered)
    }

    pub fn diff(&self, next: Option<&RenderedFrame>) -> FrameDiff {
        let previous = self.previous.as_ref();
        let previous_area = previous.map_or(Rect::new(0, 0, 0, 0), |frame| frame.area);
        let next_area = next.map_or(Rect::new(0, 0, 0, 0), |frame| frame.area);
        let previous_origin = previous.map_or(0, |frame| frame.origin_column);
        let next_origin = next.map_or(0, |frame| frame.origin_column);
        let previous_row_offset = previous.map_or(0, |frame| frame.row_offset);
        let next_row_offset = next.map_or(0, |frame| frame.row_offset);
        let width = previous_area.width.max(next_area.width);
        let height = previous_area.height.max(next_area.height);
        let mut changed_cells = 0;
        let mut changed_rows = Vec::new();
        let mut cleared_rows = Vec::new();
        let empty = Cell::EMPTY;

        for y in 0..height {
            let mut row_changed = false;
            let mut row_cleared = false;
            for x in 0..width {
                let previous_cell = previous
                    .and_then(|frame| frame.buffer.cell((x, y)))
                    .unwrap_or(&empty);
                let next_cell = next
                    .and_then(|frame| frame.buffer.cell((x, y)))
                    .unwrap_or(&empty);
                if previous_cell != next_cell {
                    changed_cells += 1;
                    row_changed = true;
                    if next.is_none() || y >= next_area.height || x >= next_area.width {
                        row_cleared = true;
                    }
                }
            }
            if row_changed {
                changed_rows.push(y);
            }
            if row_cleared {
                cleared_rows.push(y);
            }
        }

        FrameDiff {
            changed_cells,
            changed_rows,
            cleared_rows,
            previous_area,
            next_area,
            previous_origin,
            next_origin,
            previous_row_offset,
            next_row_offset,
        }
    }

    pub fn clear_previous(&mut self) -> Option<Vec<u8>> {
        let previous = self.previous.take()?;
        Some(AnsiWriter::clear_surface(
            previous.origin_column,
            previous.row_offset,
            previous.area.width,
            previous.area.height,
        ))
    }

    pub fn previous_frame(&self) -> Option<&RenderedFrame> {
        self.previous.as_ref()
    }
}

fn patch_ops(diff: &FrameDiff, frame: &RenderedFrame, force_full: bool) -> Vec<PatchOp> {
    if !force_full && !diff.changed() {
        return Vec::new();
    }
    let mut ops = vec![PatchOp::SaveCursor, PatchOp::HideCursor];
    if diff.origin_changed() && diff.previous_area.height > 0 {
        ops.push(PatchOp::ClearSurface {
            origin_column: diff.previous_origin,
            row_offset: diff.previous_row_offset,
            width: diff.previous_area.width,
            height: diff.previous_area.height,
        });
    }
    ops.push(PatchOp::MoveToSurface);
    let clear_width = frame.area.width.max(diff.previous_area.width);
    let row_count = frame.area.height.max(diff.previous_area.height);
    let repaint_all = force_full || diff.origin_changed();
    for row in 0..row_count {
        if row < frame.area.height && (repaint_all || diff.changed_rows.contains(&row)) {
            ops.push(PatchOp::ClearSpan {
                row,
                width: clear_width,
            });
            ops.push(PatchOp::PaintRow { row });
        } else if row >= frame.area.height && diff.cleared_rows.contains(&row) {
            ops.push(PatchOp::ClearSpan {
                row,
                width: diff.previous_area.width,
            });
        }
    }
    ops.extend([PatchOp::RestoreCursor, PatchOp::ShowCursor]);
    ops
}

struct AnsiWriter;

impl AnsiWriter {
    fn clear_surface(origin_column: u16, row_offset: i16, width: u16, height: u16) -> Vec<u8> {
        if width == 0 || height == 0 {
            return Vec::new();
        }
        let mut output = String::new();
        output.push_str("\x1b7\x1b[?25l");
        move_relative_rows(&mut output, row_offset);
        move_to_column(&mut output, origin_column);
        for row in 0..height {
            output.push_str("\x1b[0m");
            erase_characters(&mut output, width);
            if row + 1 < height {
                output.push_str("\x1b[1B");
                move_to_column(&mut output, origin_column);
            }
        }
        output.push_str("\x1b[0m\x1b8\x1b[?25h");
        output.into_bytes()
    }

    fn paint(frame: &RenderedFrame, diff: &FrameDiff, force_full: bool) -> Vec<u8> {
        if !force_full && !diff.changed() {
            return Vec::new();
        }
        let mut output = String::new();
        output.push_str("\x1b7\x1b[?25l");
        if diff.origin_changed() && diff.previous_area.height > 0 {
            output.push_str("\x1b8");
            clear_surface_body(
                &mut output,
                diff.previous_origin,
                diff.previous_row_offset,
                diff.previous_area.width,
                diff.previous_area.height,
            );
            output.push_str("\x1b8");
        }
        move_relative_rows(&mut output, frame.row_offset);
        move_to_column(&mut output, frame.origin_column);
        let clear_width = frame.area.width.max(diff.previous_area.width);
        let row_count = frame.area.height.max(diff.previous_area.height);
        let repaint_all = force_full || diff.origin_changed();
        for row in 0..row_count {
            if row < frame.area.height && (repaint_all || diff.changed_rows.contains(&row)) {
                output.push_str("\x1b[0m");
                erase_characters(&mut output, clear_width);
                write_row(&mut output, frame, row);
            } else if row >= frame.area.height && diff.cleared_rows.contains(&row) {
                output.push_str("\x1b[0m");
                erase_characters(&mut output, diff.previous_area.width);
            }
            if row + 1 < row_count {
                output.push_str("\x1b[1B");
                move_to_column(&mut output, frame.origin_column);
            }
        }
        output.push_str("\x1b[0m\x1b8\x1b[?25h");
        output.into_bytes()
    }
}

fn move_relative_rows(output: &mut String, offset: i16) {
    if offset > 0 {
        let _ = write!(output, "\x1b[{}B", offset);
    } else if offset < 0 {
        let _ = write!(output, "\x1b[{}A", offset.unsigned_abs());
    }
}

fn move_to_column(output: &mut String, column: u16) {
    let _ = write!(output, "\x1b[{}G", column.saturating_add(1));
}

fn clear_surface_body(
    output: &mut String,
    origin_column: u16,
    row_offset: i16,
    width: u16,
    height: u16,
) {
    move_relative_rows(output, row_offset);
    move_to_column(output, origin_column);
    for row in 0..height {
        output.push_str("\x1b[0m");
        erase_characters(output, width);
        if row + 1 < height {
            output.push_str("\x1b[1B");
            move_to_column(output, origin_column);
        }
    }
}

fn erase_characters(output: &mut String, width: u16) {
    let _ = write!(output, "\x1b[{}X", width);
}

fn write_row(output: &mut String, frame: &RenderedFrame, row: u16) {
    let mut active_style = Style::default();
    let mut previous_wide = false;
    let empty = Cell::EMPTY;
    for x in 0..frame.area.width {
        let cell = frame.buffer.cell((x, row)).unwrap_or(&empty);
        if previous_wide {
            previous_wide = false;
            continue;
        }
        set_style(output, &mut active_style, cell.style());
        output.push_str(cell.symbol());
        previous_wide = cell.symbol().width() > 1;
    }
    output.push_str("\x1b[0m");
}

fn set_style(output: &mut String, current: &mut Style, next: Style) {
    if *current == next {
        return;
    }
    output.push_str("\x1b[0m");
    if let Some(color) = next.fg {
        write_color(output, color, false);
    }
    if let Some(color) = next.bg {
        write_color(output, color, true);
    }
    let modifiers = next.add_modifier;
    let modifier_codes = [
        (Modifier::BOLD, "1"),
        (Modifier::DIM, "2"),
        (Modifier::ITALIC, "3"),
        (Modifier::UNDERLINED, "4"),
        (Modifier::SLOW_BLINK, "5"),
        (Modifier::RAPID_BLINK, "6"),
        (Modifier::REVERSED, "7"),
        (Modifier::HIDDEN, "8"),
        (Modifier::CROSSED_OUT, "9"),
    ];
    for (modifier, code) in modifier_codes {
        if modifiers.contains(modifier) {
            let _ = write!(output, "\x1b[{}m", code);
        }
    }
    *current = next;
}

fn write_color(output: &mut String, color: Color, background: bool) {
    let prefix = if background { 48 } else { 38 };
    match color {
        Color::Reset => {
            let _ = write!(output, "\x1b[{}m", if background { 49 } else { 39 });
        }
        Color::Black => output.push_str(if background { "\x1b[40m" } else { "\x1b[30m" }),
        Color::Red => output.push_str(if background { "\x1b[41m" } else { "\x1b[31m" }),
        Color::Green => output.push_str(if background { "\x1b[42m" } else { "\x1b[32m" }),
        Color::Yellow => output.push_str(if background { "\x1b[43m" } else { "\x1b[33m" }),
        Color::Blue => output.push_str(if background { "\x1b[44m" } else { "\x1b[34m" }),
        Color::Magenta => output.push_str(if background { "\x1b[45m" } else { "\x1b[35m" }),
        Color::Cyan => output.push_str(if background { "\x1b[46m" } else { "\x1b[36m" }),
        Color::Gray => output.push_str(if background { "\x1b[47m" } else { "\x1b[37m" }),
        Color::DarkGray => output.push_str(if background { "\x1b[100m" } else { "\x1b[90m" }),
        Color::LightRed => output.push_str(if background { "\x1b[101m" } else { "\x1b[91m" }),
        Color::LightGreen => output.push_str(if background { "\x1b[102m" } else { "\x1b[92m" }),
        Color::LightYellow => output.push_str(if background { "\x1b[103m" } else { "\x1b[93m" }),
        Color::LightBlue => output.push_str(if background { "\x1b[104m" } else { "\x1b[94m" }),
        Color::LightMagenta => output.push_str(if background { "\x1b[105m" } else { "\x1b[95m" }),
        Color::LightCyan => output.push_str(if background { "\x1b[106m" } else { "\x1b[96m" }),
        Color::White => output.push_str(if background { "\x1b[107m" } else { "\x1b[97m" }),
        Color::Indexed(value) => {
            let _ = write!(output, "\x1b[{};5;{}m", prefix, value);
        }
        Color::Rgb(red, green, blue) => {
            let _ = write!(output, "\x1b[{};2;{};{};{}m", prefix, red, green, blue);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keel_ui::{PopupItem, SuggestionPopup};

    fn strip_ansi(input: &str) -> String {
        let mut output = String::with_capacity(input.len());
        let mut escape = false;
        let mut csi = false;

        for byte in input.bytes() {
            if csi {
                if (0x40..=0x7e).contains(&byte) {
                    csi = false;
                }
                continue;
            }
            if escape {
                if byte == b'[' {
                    csi = true;
                }
                escape = false;
                continue;
            }
            if byte == 0x1b {
                escape = true;
            } else {
                output.push(byte as char);
            }
        }

        output
    }

    fn popup(items: &[&str]) -> Scene {
        Scene::new(
            SuggestionPopup::new(
                format!("1/{}; Tab to accept", items.len()),
                items.iter().map(|item| PopupItem::new(*item, "")).collect(),
                Some(0),
            )
            .query("git"),
        )
    }

    #[test]
    fn first_frame_paints_every_row_into_one_transaction() {
        let mut renderer = Renderer::new();
        let rendered = renderer
            .render(
                &popup(&["run git", "inspect git"]),
                RenderContext::new(80, 12, 3).full_repaint(true),
            )
            .unwrap();
        assert_eq!(
            rendered.diff.changed_rows,
            (0..rendered.frame.area.height).collect::<Vec<_>>()
        );
        assert_eq!(rendered.transaction.ops.first(), Some(&PatchOp::SaveCursor));
        let payload = String::from_utf8(rendered.transaction.payload).unwrap();
        let visible = strip_ansi(&payload);
        assert!(visible.contains("run git"));
        assert!(visible.contains("1/2; Tab to accept"));
        assert!(!visible.contains("Keel suggestions"));
    }

    #[test]
    fn unchanged_frame_has_no_changed_rows_but_can_be_forced_full() {
        let mut renderer = Renderer::new();
        let scene = popup(&["run git"]);
        let context = RenderContext::new(80, 12, 3).full_repaint(true);
        renderer.render(&scene, context).unwrap();
        let unchanged = renderer
            .render(&scene, RenderContext::new(80, 12, 3))
            .unwrap();
        assert!(unchanged.diff.changed_rows.is_empty());
        assert!(!unchanged.diff.changed());
        assert!(unchanged.transaction.payload.is_empty());
        let forced = renderer
            .render(&scene, RenderContext::new(80, 12, 3).full_repaint(true))
            .unwrap();
        assert!(!forced.transaction.payload.is_empty());
    }

    #[test]
    fn moving_surface_clears_old_anchor_and_repaints_new_anchor() {
        let mut renderer = Renderer::new();
        let scene = popup(&["run git"]);
        renderer
            .render(&scene, RenderContext::new(80, 12, 2).full_repaint(true))
            .unwrap();
        let moved = renderer
            .render(&scene, RenderContext::new(80, 12, 7))
            .unwrap();

        assert!(moved.diff.origin_changed());
        assert!(moved.transaction.ops.iter().any(|op| {
            matches!(
                op,
                PatchOp::ClearSurface {
                    origin_column: 2,
                    ..
                }
            )
        }));
        let payload = String::from_utf8(moved.transaction.payload).unwrap();
        assert!(payload.contains("\x1b[3G"));
        assert!(payload.contains("\x1b[8G"));
    }

    #[test]
    fn moving_surface_restores_host_cursor_before_clearing_the_old_offset() {
        let mut renderer = Renderer::new();
        let scene = popup(&["run git"]);
        renderer
            .render(
                &scene,
                RenderContext::new(80, 12, 2)
                    .row_offset(-4)
                    .full_repaint(true),
            )
            .unwrap();
        let moved = renderer
            .render(
                &scene,
                RenderContext::new(80, 12, 7)
                    .row_offset(2)
                    .full_repaint(false),
            )
            .unwrap();
        let payload = String::from_utf8(moved.transaction.payload).unwrap();
        let old_clear = payload.find("\x1b8\x1b[4A").unwrap();
        let new_paint = payload.find("\x1b8\x1b[2B").unwrap();
        assert!(old_clear < new_paint);
    }

    #[test]
    fn shrinking_surface_clears_old_rows() {
        let mut renderer = Renderer::new();
        let previous = renderer
            .render(
                &popup(&["one", "two", "three"]),
                RenderContext::new(80, 12, 0),
            )
            .unwrap();
        let next = renderer
            .render(&popup(&["one"]), RenderContext::new(80, 12, 0))
            .unwrap();
        assert_eq!(
            next.diff.cleared_rows,
            (next.frame.area.height..previous.frame.area.height).collect::<Vec<_>>()
        );
        let clear = renderer.clear_previous().unwrap();
        assert!(String::from_utf8(clear).unwrap().contains("\x1b["));
    }

    #[test]
    fn encoder_uses_relative_cursor_restore_and_no_full_screen_clear() {
        let mut renderer = Renderer::new();
        let payload = renderer
            .render(
                &popup(&["run echo hi"]),
                RenderContext::new(40, 8, 5).full_repaint(true),
            )
            .unwrap()
            .transaction
            .payload;
        let payload = String::from_utf8(payload).unwrap();
        assert!(payload.starts_with("\x1b7\x1b[?25l\x1b[1B\x1b[6G"));
        assert!(payload.contains("\x1b[0m\x1b8\x1b[?25h"));
        assert!(!payload.contains(" q"));
        assert!(!payload.contains("\x1b[2J"));
    }

    #[test]
    fn rendered_region_reports_the_reserved_height() {
        let mut renderer = Renderer::new();
        let rendered = renderer
            .render(
                &popup(&["run git"]),
                RenderContext::new(80, 12, 0).full_repaint(true),
            )
            .unwrap();
        assert_eq!(rendered.used_rows, rendered.frame.area.height);
    }

    #[test]
    fn default_popup_uses_only_terminal_palette_colors() {
        let mut renderer = Renderer::new();
        let payload = renderer
            .render(
                &popup(&["run git", "inspect git"]),
                RenderContext::new(80, 12, 0).full_repaint(true),
            )
            .unwrap()
            .transaction
            .payload;
        let payload = String::from_utf8(payload).unwrap();

        assert!(!payload.contains("38;"));
        assert!(!payload.contains("48;"));
        assert!(payload.contains("\x1b[32m"));
        assert!(payload.contains("\x1b[97m"));
        assert!(payload.contains("\x1b[1m"));
        assert!(payload.contains("\x1b[4m"));
        assert!(payload.contains("\x1b[7m"));
    }

    #[test]
    fn above_cursor_surface_uses_negative_row_offset_without_owning_cursor_style() {
        let mut renderer = Renderer::new();
        let rendered = renderer
            .render(
                &popup(&["run git"]),
                RenderContext::new(80, 5, 8)
                    .row_offset(-5)
                    .full_repaint(true),
            )
            .unwrap();
        assert_eq!(rendered.frame.row_offset, -5);
        let payload = String::from_utf8(rendered.transaction.payload).unwrap();
        assert!(payload.starts_with("\x1b7\x1b[?25l\x1b[5A"));
        assert!(!payload.contains(" q"));
    }
}
