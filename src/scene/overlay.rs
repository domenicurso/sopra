use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
};
use unicode_width::UnicodeWidthStr;

use super::{
    DemoItem, DemoItemKind, Element, MAX_OVERLAY_WIDTH, OVERLAY_FOOTER,
    canvas::{Canvas, TextRun},
};

pub(super) struct OverlayElement {
    pub(super) area: Rect,
    pub(super) items: Vec<DemoItem>,
    pub(super) selected: usize,
    pub(super) query: String,
    pub(super) connector_column: u16,
    pub(super) connector: &'static str,
}

impl Element for OverlayElement {
    fn paint(&self, canvas: &mut Canvas) {
        if self.area.width < 8 || self.area.height < 3 {
            return;
        }
        let border = Style::default().fg(Color::Rgb(91, 151, 190));
        let inner = self.inner_area();
        canvas.border(self.area, border);
        self.paint_connector(canvas, border);
        paint_title(canvas, self.area);
        self.paint_items(canvas, inner);
        paint_footer(canvas, self.area);
    }
}

impl OverlayElement {
    fn inner_area(&self) -> Rect {
        Rect::new(
            self.area.left().saturating_add(1),
            self.area.top().saturating_add(1),
            self.area.width.saturating_sub(2),
            self.area.height.saturating_sub(2),
        )
    }

    fn paint_connector(&self, canvas: &mut Canvas, style: Style) {
        let row = if self.connector == "┬" {
            self.area.bottom().saturating_sub(1)
        } else {
            self.area.top()
        };
        canvas.put(
            self.area.left().saturating_add(self.connector_column),
            row,
            self.connector,
            style,
        );
    }

    fn paint_items(&self, canvas: &mut Canvas, inner: Rect) {
        let visible = inner.height.max(1) as usize;
        let start = self
            .selected
            .saturating_sub(visible.saturating_sub(1))
            .min(self.items.len().saturating_sub(visible));
        for (offset, item) in self.items[start..].iter().take(visible).enumerate() {
            let row = inner.top().saturating_add(offset as u16);
            let selected = start + offset == self.selected;
            paint_item(
                canvas,
                OverlayRow {
                    inner,
                    row,
                    item: *item,
                    selected,
                    query: &self.query,
                },
            );
        }
    }
}

fn paint_title(canvas: &mut Canvas, area: Rect) {
    canvas.text(TextRun {
        position: (area.left().saturating_add(2), area.top()),
        text: " demo ",
        style: Style::default()
            .fg(Color::Rgb(130, 220, 190))
            .add_modifier(Modifier::BOLD),
        max_width: area.width.saturating_sub(4),
    });
}

struct OverlayRow<'a> {
    inner: Rect,
    row: u16,
    item: DemoItem,
    selected: bool,
    query: &'a str,
}

fn paint_item(canvas: &mut Canvas, row: OverlayRow<'_>) {
    let style = item_style(row.item.kind, row.selected);
    let label = if row.selected {
        format!("▸ {}", row.item.label)
    } else {
        format!("  {}", row.item.label)
    };
    canvas.text(TextRun {
        position: (row.inner.left(), row.row),
        text: &label,
        style,
        max_width: row.inner.width,
    });
    highlight_query(
        canvas,
        QueryHighlight {
            position: (row.inner.left(), row.row),
            label: &label,
            query: row.query,
        },
    );
}

fn item_style(kind: DemoItemKind, selected: bool) -> Style {
    let color = match kind {
        DemoItemKind::Command => Color::Rgb(125, 196, 255),
        DemoItemKind::File => Color::Rgb(248, 195, 111),
        DemoItemKind::Help => Color::Rgb(193, 151, 255),
    };
    Style::default().fg(color).add_modifier(if selected {
        Modifier::BOLD
    } else {
        Modifier::empty()
    })
}

fn paint_footer(canvas: &mut Canvas, area: Rect) {
    canvas.text(TextRun {
        position: (
            area.left().saturating_add(2),
            area.bottom().saturating_sub(1),
        ),
        text: OVERLAY_FOOTER,
        style: Style::default()
            .fg(Color::Rgb(129, 142, 160))
            .add_modifier(Modifier::DIM),
        max_width: area.width.saturating_sub(4),
    });
}

struct QueryHighlight<'a> {
    position: (u16, u16),
    label: &'a str,
    query: &'a str,
}

fn highlight_query(canvas: &mut Canvas, highlight: QueryHighlight<'_>) {
    if highlight.query.is_empty() {
        return;
    }
    let query = highlight.query.to_ascii_lowercase();
    let label_lower = highlight.label.to_ascii_lowercase();
    let Some(start) = label_lower.find(&query) else {
        return;
    };
    let prefix = &highlight.label[..start];
    let matched = &highlight.label[start..start + query.len()];
    canvas.text(TextRun {
        position: (
            highlight
                .position
                .0
                .saturating_add(UnicodeWidthStr::width(prefix) as u16),
            highlight.position.1,
        ),
        text: matched,
        style: Style::default()
            .fg(Color::Rgb(111, 214, 176))
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        max_width: UnicodeWidthStr::width(matched) as u16,
    });
}

pub(super) fn overlay_width(items: &[DemoItem], terminal_width: u16) -> u16 {
    if terminal_width <= 2 {
        return terminal_width.max(1);
    }
    let content_width = items
        .iter()
        .map(|item| UnicodeWidthStr::width(item.label) + 2)
        .chain(std::iter::once(UnicodeWidthStr::width(OVERLAY_FOOTER)))
        .max()
        .unwrap_or(1)
        .saturating_add(4) as u16;
    content_width
        .min(MAX_OVERLAY_WIDTH)
        .min(terminal_width.saturating_sub(2))
}
