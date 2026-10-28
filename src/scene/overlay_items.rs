use super::canvas::{Canvas, TextRun};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
};
use sopra_completion::{CompletionItem, CompletionKind, CompletionSource};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub(super) struct ItemRow<'a> {
    pub(super) item: &'a CompletionItem,
    pub(super) selected: bool,
    pub(super) query: &'a str,
}

impl ItemRow<'_> {
    pub(super) fn paint(self, canvas: &mut Canvas, inner: Rect, row: u16) {
        let base = item_style(self.item.kind, self.selected);
        let available = inner.width;
        let label_width = UnicodeWidthStr::width(self.item.display.as_str()) as u16;
        let detail = item_detail(self.item);
        let history = self.item.kind == CompletionKind::History;
        let metadata_width = if history && available > 12 {
            (UnicodeWidthStr::width(detail.as_str()) as u16).min(available / 2)
        } else {
            0
        };
        let reserve = if metadata_width > 0 {
            metadata_width + 2
        } else {
            0
        };
        super::overlay_label::paint(
            canvas,
            self.item,
            self.query,
            TextRun {
                position: (inner.left(), row),
                text: &self.item.display,
                style: base,
                max_width: label_width.min(available.saturating_sub(reserve)),
            },
        );
        let (column, width) = if history {
            (inner.right().saturating_sub(metadata_width), metadata_width)
        } else if label_width.saturating_add(2) < available {
            (inner.left() + label_width + 2, available - label_width - 2)
        } else {
            return;
        };
        if !detail.is_empty() {
            paint_detail(
                canvas,
                TextRun {
                    position: (column, row),
                    text: &detail,
                    style: Style::default().add_modifier(Modifier::DIM),
                    max_width: width,
                },
            );
        }
    }
}

fn paint_detail(canvas: &mut Canvas, run: TextRun) {
    let (column, row) = run.position;
    let (detail, max_width, style) = (run.text, run.max_width, run.style);
    if max_width == 0 {
        return;
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_age_stays_at_the_right_edge_when_the_command_is_truncated() {
        let area = Rect::new(0, 0, 30, 1);
        let mut canvas = Canvas::new(area);
        let item = CompletionItem::new(
            "echo a very long command that must be clipped",
            "5m ago",
            "ignored",
            CompletionKind::History,
        );
        ItemRow {
            item: &item,
            selected: true,
            query: "echo",
        }
        .paint(&mut canvas, area, 0);
        let text = canvas
            .buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(text.ends_with("5m ago"), "{text}");
        assert!(text.contains('…'), "{text}");
        assert!(canvas.buffer[(24, 0)].modifier.contains(Modifier::DIM));
    }
}
