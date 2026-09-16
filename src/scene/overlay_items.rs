use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::completion::{CompletionItem, CompletionKind};

use super::canvas::{Canvas, TextRun};

pub(super) fn paint_item(
    canvas: &mut Canvas,
    inner: Rect,
    row: u16,
    item: &CompletionItem,
    selected: bool,
) {
    let style = item_style(item.kind, selected);
    canvas.put(inner.left(), row, if selected { "▸" } else { " " }, style);
    let label_column = inner.left().saturating_add(2);
    let available = inner.right().saturating_sub(label_column);
    let label_width = paint_label(canvas, label_column, row, item, style, available);
    let detail = item
        .description
        .as_deref()
        .or_else(|| item.location.as_deref().and_then(|path| path.to_str()));
    if let Some(detail) = detail {
        let detail_column = label_column.saturating_add(label_width).saturating_add(2);
        paint_detail(
            canvas,
            detail_column,
            row,
            detail,
            selected,
            inner.right().saturating_sub(detail_column),
        );
    }
}

fn paint_label(
    canvas: &mut Canvas,
    column: u16,
    row: u16,
    item: &CompletionItem,
    base: Style,
    max_width: u16,
) -> u16 {
    let mut offset = 0_u16;
    for (byte_offset, grapheme) in item.display.grapheme_indices(true) {
        let width = UnicodeWidthStr::width(grapheme) as u16;
        if width == 0 {
            continue;
        }
        if offset.saturating_add(width) > max_width {
            if max_width > 0 {
                canvas.put(column.saturating_add(max_width - 1), row, "…", base);
            }
            return max_width;
        }
        let style = if item.match_indices.contains(&byte_offset) {
            base.fg(Color::Green)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            base
        };
        canvas.text(TextRun {
            position: (column.saturating_add(offset), row),
            text: grapheme,
            style,
            max_width: max_width.saturating_sub(offset),
        });
        offset = offset.saturating_add(width);
    }
    offset
}

fn paint_detail(
    canvas: &mut Canvas,
    column: u16,
    row: u16,
    detail: &str,
    selected: bool,
    max_width: u16,
) {
    if max_width == 0 {
        return;
    }
    let mut style = Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::DIM);
    if selected {
        style = style.add_modifier(Modifier::REVERSED);
    }
    let width = UnicodeWidthStr::width(detail) as u16;
    if width <= max_width {
        canvas.text(TextRun {
            position: (column, row),
            text: detail,
            style,
            max_width,
        });
        return;
    }
    let ellipsis_width = 1;
    let mut offset = 0_u16;
    for grapheme in detail.graphemes(true) {
        let width = UnicodeWidthStr::width(grapheme) as u16;
        if width == 0 || offset.saturating_add(width).saturating_add(ellipsis_width) > max_width {
            break;
        }
        canvas.text(TextRun {
            position: (column.saturating_add(offset), row),
            text: grapheme,
            style,
            max_width: max_width.saturating_sub(offset),
        });
        offset = offset.saturating_add(width);
    }
    canvas.put(column.saturating_add(offset), row, "…", style);
}

pub(super) fn paint_scrollbar(
    canvas: &mut Canvas,
    inner: Rect,
    start: usize,
    visible: usize,
    total: usize,
) {
    let track = Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM);
    let thumb = Style::default().fg(Color::Green);
    let thumb_height = (visible.saturating_mul(visible) / total).max(1);
    let thumb_start = start.saturating_mul(visible) / total;
    for offset in 0..visible {
        let style = if (thumb_start..thumb_start.saturating_add(thumb_height)).contains(&offset) {
            thumb
        } else {
            track
        };
        canvas.put(
            inner.right().saturating_sub(1),
            inner.top() + offset as u16,
            "│",
            style,
        );
    }
}

fn item_style(kind: CompletionKind, selected: bool) -> Style {
    let color = match kind {
        CompletionKind::Generic
        | CompletionKind::Command
        | CompletionKind::Alias
        | CompletionKind::Function
        | CompletionKind::Builtin
        | CompletionKind::Option
        | CompletionKind::Subcommand
        | CompletionKind::Value
        | CompletionKind::Positional
        | CompletionKind::Host
        | CompletionKind::User => Color::Reset,
        CompletionKind::File => Color::Yellow,
        CompletionKind::Directory => Color::Blue,
    };
    Style::default().fg(color).add_modifier(if selected {
        Modifier::BOLD | Modifier::REVERSED
    } else {
        Modifier::empty()
    })
}
