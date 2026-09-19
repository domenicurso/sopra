use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use sopra_completion::{CompletionItem, CompletionKind, CompletionSource};

use super::{
    canvas::{Canvas, TextRun},
    overlay_scrollbar,
};

pub(super) fn paint_item(
    canvas: &mut Canvas,
    inner: Rect,
    row: u16,
    item: &CompletionItem,
    selected: bool,
    query: &str,
) {
    let base = item_style(item.kind, selected);
    let label_column = inner.left();
    let available = inner.right().saturating_sub(label_column);
    let label_width = UnicodeWidthStr::width(item.display.as_str()) as u16;
    let label_limit = label_width.min(available);
    paint_label(canvas, label_column, row, item, query, base, label_limit);
    let detail = item_detail(item);
    if detail.is_empty() || label_width.saturating_add(2) >= available {
        return;
    }
    let detail_column = label_column.saturating_add(label_width).saturating_add(2);
    paint_detail(
        canvas,
        detail_column,
        row,
        &detail,
        available.saturating_sub(label_width).saturating_sub(2),
    );
}

fn paint_label(
    canvas: &mut Canvas,
    column: u16,
    row: u16,
    item: &CompletionItem,
    query: &str,
    base: Style,
    max_width: u16,
) {
    let fallback = fuzzy_positions(&item.display, query);
    let mut offset = 0_u16;
    for (index, (byte_offset, grapheme)) in item.display.grapheme_indices(true).enumerate() {
        let width = UnicodeWidthStr::width(grapheme) as u16;
        if width == 0 {
            continue;
        }
        if offset.saturating_add(width) > max_width {
            if max_width > 0 {
                canvas.put(column + max_width - 1, row, "…", base);
            }
            return;
        }
        let matched = if item.match_indices.is_empty() {
            fallback.get(index).copied().unwrap_or(false)
        } else {
            item.match_indices.contains(&byte_offset)
        };
        canvas.text(TextRun {
            position: (column + offset, row),
            text: grapheme,
            style: matched_style(base, matched),
            max_width: max_width.saturating_sub(offset),
        });
        offset = offset.saturating_add(width);
    }
}

fn paint_detail(canvas: &mut Canvas, column: u16, row: u16, detail: &str, max_width: u16) {
    if max_width == 0 {
        return;
    }
    let style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::DIM);
    if UnicodeWidthStr::width(detail) as u16 <= max_width {
        canvas.text(TextRun {
            position: (column, row),
            text: detail,
            style,
            max_width,
        });
        return;
    }
    let mut offset = 0_u16;
    for grapheme in detail.graphemes(true) {
        let width = UnicodeWidthStr::width(grapheme) as u16;
        if width == 0 || offset.saturating_add(width).saturating_add(1) > max_width {
            break;
        }
        canvas.text(TextRun {
            position: (column + offset, row),
            text: grapheme,
            style,
            max_width: max_width - offset,
        });
        offset += width;
    }
    canvas.put(column + offset, row, "…", style);
}

pub(super) fn paint_scrollbar(
    canvas: &mut Canvas,
    area: Rect,
    start: usize,
    visible: usize,
    total: usize,
) {
    overlay_scrollbar::paint(canvas, area, start, visible, total);
}

pub(super) fn item_detail(item: &CompletionItem) -> String {
    item.description
        .clone()
        .or_else(|| {
            if matches!(item.source, CompletionSource::Filesystem) {
                None
            } else {
                item.location
                    .as_ref()
                    .map(|path| path.display().to_string())
            }
        })
        .unwrap_or_default()
}

fn matched_style(base: Style, matched: bool) -> Style {
    if matched {
        base.fg(Color::Green)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        base
    }
}

fn fuzzy_positions(text: &str, query: &str) -> Vec<bool> {
    let graphemes = text.graphemes(true).collect::<Vec<_>>();
    let mut matched = vec![false; graphemes.len()];
    let mut search_start = 0;
    for wanted in query
        .rsplit('/')
        .next()
        .unwrap_or(query)
        .graphemes(true)
        .filter(|value| !value.chars().all(char::is_whitespace))
    {
        let Some(offset) = graphemes[search_start..]
            .iter()
            .position(|value| value.eq_ignore_ascii_case(wanted))
            .map(|index| index + search_start)
        else {
            continue;
        };
        matched[offset] = true;
        search_start = offset + 1;
    }
    matched
}

fn item_style(kind: CompletionKind, selected: bool) -> Style {
    let color = match kind {
        CompletionKind::File => Color::LightYellow,
        CompletionKind::Directory => Color::LightBlue,
        _ => Color::Reset,
    };
    let mut style = Style::default().fg(color);
    if kind == CompletionKind::Directory {
        style = style.add_modifier(Modifier::BOLD);
    }
    if selected {
        style.add_modifier(Modifier::REVERSED)
    } else {
        style
    }
}
