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
    let label_width = UnicodeWidthStr::width(item.label.as_str()) as u16;
    paint_label(canvas, label_column, row, item, style, available);
    let detail_width = UnicodeWidthStr::width(item.detail.as_str()) as u16;
    if !item.detail.is_empty()
        && label_width.saturating_add(detail_width).saturating_add(2) <= available
    {
        let detail_column = label_column.saturating_add(label_width).saturating_add(2);
        canvas.text(TextRun {
            position: (detail_column, row),
            text: &item.detail,
            style: Style::default()
                .fg(Color::Rgb(129, 142, 160))
                .add_modifier(Modifier::DIM),
            max_width: inner.right().saturating_sub(detail_column),
        });
    }
}

fn paint_label(
    canvas: &mut Canvas,
    column: u16,
    row: u16,
    item: &CompletionItem,
    base: Style,
    max_width: u16,
) {
    let mut offset = 0_u16;
    for (byte_offset, grapheme) in item.label.grapheme_indices(true) {
        let width = UnicodeWidthStr::width(grapheme) as u16;
        if width == 0 {
            continue;
        }
        if offset.saturating_add(width) > max_width {
            if max_width > 0 {
                canvas.put(column.saturating_add(max_width - 1), row, "…", base);
            }
            break;
        }
        let style = if item.match_indices.contains(&byte_offset) {
            Style::default()
                .fg(Color::Rgb(111, 214, 176))
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
}

pub(super) fn paint_scrollbar(
    canvas: &mut Canvas,
    inner: Rect,
    start: usize,
    visible: usize,
    total: usize,
) {
    let track = Style::default()
        .fg(Color::Rgb(91, 151, 190))
        .add_modifier(Modifier::DIM);
    let thumb = Style::default().fg(Color::Rgb(130, 220, 190));
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
        CompletionKind::Generic => Color::Rgb(220, 224, 232),
        CompletionKind::File => Color::Rgb(248, 195, 111),
        CompletionKind::Directory => Color::Rgb(125, 196, 255),
    };
    Style::default().fg(color).add_modifier(if selected {
        Modifier::BOLD
    } else {
        Modifier::empty()
    })
}
