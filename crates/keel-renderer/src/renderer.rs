use keel_ui::{Constraints, Scene};
use ratatui::{buffer::Buffer, layout::Rect};

use crate::ansi::AnsiWriter;
use crate::model::{
    FrameDiff, PatchOp, RenderContext, RenderError, RenderTransaction, RenderedFrame,
    RenderedRegion,
};

#[derive(Debug, Default)]
pub struct Renderer {
    pub(crate) previous: Option<RenderedFrame>,
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
        let scroll_rows = context.scroll_rows.min(context.cursor_row);
        let cursor_row = context.cursor_row.saturating_sub(scroll_rows);
        let anchor_row = surface_row(cursor_row, context.row_offset);
        let frame = RenderedFrame {
            area,
            used_size: keel_ui::Size::new(width, height),
            buffer,
            terminal_columns: context.terminal_columns,
            terminal_rows: context.terminal_rows,
            origin_column,
            cursor_row,
            anchor_row,
            row_offset: context.row_offset,
        };
        let diff = self.diff(Some(&frame));
        let payload = AnsiWriter::paint(
            &frame,
            &diff,
            context.force_full,
            context.cursor_row,
            scroll_rows,
        );
        let transaction = RenderTransaction {
            origin_column,
            row_offset: context.row_offset,
            width,
            height,
            scroll_rows,
            ops: patch_ops(
                &diff,
                &frame,
                context.force_full,
                context.cursor_row,
                scroll_rows,
            ),
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

    pub fn clear_previous(&mut self) -> Option<Vec<u8>> {
        let cursor_row = self.previous.as_ref()?.cursor_row;
        self.clear_previous_at(cursor_row)
    }

    pub fn clear_previous_at(&mut self, cursor_row: u16) -> Option<Vec<u8>> {
        let previous = self.previous.take()?;
        Some(AnsiWriter::clear_surface(
            previous.origin_column,
            clear_row_offset(&previous, cursor_row),
            previous.area.width,
            previous.area.height,
        ))
    }

    pub fn previous_frame(&self) -> Option<&RenderedFrame> {
        self.previous.as_ref()
    }
}

fn surface_row(cursor_row: u16, row_offset: i16) -> u16 {
    if row_offset >= 0 {
        cursor_row.saturating_add(row_offset as u16)
    } else {
        cursor_row.saturating_sub(row_offset.unsigned_abs())
    }
}

fn row_delta(surface_row: u16, cursor_row: u16) -> i16 {
    (i32::from(surface_row) - i32::from(cursor_row)).clamp(i32::from(i16::MIN), i32::from(i16::MAX))
        as i16
}

fn clear_row_offset(frame: &RenderedFrame, cursor_row: u16) -> i16 {
    if frame.cursor_row == cursor_row {
        frame.row_offset
    } else {
        row_delta(frame.anchor_row, cursor_row)
    }
}

fn clear_diff_row_offset(diff: &FrameDiff, cursor_row: u16) -> i16 {
    if diff.previous_cursor_row == cursor_row {
        diff.previous_row_offset
    } else {
        row_delta(diff.previous_anchor_row, cursor_row)
    }
}

fn patch_ops(
    diff: &FrameDiff,
    frame: &RenderedFrame,
    force_full: bool,
    cursor_row: u16,
    scroll_rows: u16,
) -> Vec<PatchOp> {
    if !force_full && !diff.changed() && scroll_rows == 0 {
        return Vec::new();
    }
    let mut ops = vec![PatchOp::SaveCursor, PatchOp::HideCursor];
    if diff.origin_changed() && diff.previous_area.height > 0 {
        ops.push(PatchOp::ClearSurface {
            origin_column: diff.previous_origin,
            row_offset: clear_diff_row_offset(diff, cursor_row),
            width: diff.previous_area.width,
            height: diff.previous_area.height,
        });
    }
    if scroll_rows > 0 {
        ops.push(PatchOp::ScrollUp { rows: scroll_rows });
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
