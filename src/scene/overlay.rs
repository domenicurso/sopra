use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
};
use unicode_width::UnicodeWidthStr;

use sopra_completion::CompletionItem;

use super::{
    Element,
    canvas::{Canvas, TextRun},
    overlay_items::item_detail,
};

pub(super) struct OverlayElement {
    pub(super) area: Rect,
    pub(super) items: Vec<CompletionItem>,
    pub(super) selected: Option<usize>,
    pub(super) viewport_start: usize,
    pub(super) item_queries: Vec<String>,
    pub(super) footer_hint: String,
    pub(super) footer: String,
    pub(super) connector_column: u16,
    pub(super) connector: &'static str,
}

impl Element for OverlayElement {
    fn paint(&self, canvas: &mut Canvas) {
        if self.area.width < 8 || self.area.height < 3 {
            return;
        }
        let border = Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::DIM);
        let inner = self.inner_area();
        canvas.border(self.area, border);
        self.paint_connector(canvas, border);
        self.paint_items(canvas, inner);
        paint_footer(canvas, self.area, &self.footer_hint, &self.footer);
    }
}

impl OverlayElement {
    fn inner_area(&self) -> Rect {
        Rect::new(
            self.area.left().saturating_add(2),
            self.area.top().saturating_add(1),
            self.area.width.saturating_sub(4),
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
        let start = viewport_start(
            self.items.len(),
            self.selected,
            self.viewport_start,
            visible,
        );
        for (offset, item) in self.items[start..].iter().take(visible).enumerate() {
            let row = inner.top().saturating_add(offset as u16);
            super::overlay_items::paint_item(
                canvas,
                inner,
                row,
                item,
                self.selected == Some(start + offset),
                self.item_queries
                    .get(start + offset)
                    .map_or("", String::as_str),
            );
        }
        if self.items.len() > visible {
            super::overlay_items::paint_scrollbar(
                canvas,
                Rect::new(
                    self.area.right().saturating_sub(1),
                    inner.top(),
                    1,
                    inner.height,
                ),
                start,
                visible,
                self.items.len(),
            );
        }
    }
}

fn viewport_start(
    total: usize,
    selected: Option<usize>,
    requested: usize,
    visible: usize,
) -> usize {
    let maximum = total.saturating_sub(visible);
    let selected = selected.unwrap_or_default();
    requested
        .max(selected.saturating_add(1).saturating_sub(visible))
        .min(maximum)
}

fn paint_footer(canvas: &mut Canvas, area: Rect, hint: &str, footer: &str) {
    let right_width = UnicodeWidthStr::width(footer) as u16;
    let left = area.left().saturating_add(2);
    let right = area.right().saturating_sub(1).saturating_sub(right_width);
    let hint_width = right.saturating_sub(left).saturating_sub(1);
    let style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::DIM);
    canvas.text(TextRun {
        position: (left, area.bottom().saturating_sub(1)),
        text: hint,
        style,
        max_width: hint_width,
    });
    canvas.text(TextRun {
        position: (right, area.bottom().saturating_sub(1)),
        text: footer,
        style,
        max_width: right_width,
    });
}

pub(super) fn overlay_width(
    items: &[CompletionItem],
    hint: &str,
    footer: &str,
    terminal_width: u16,
) -> u16 {
    if terminal_width <= 2 {
        return terminal_width.max(1);
    }
    let desired_width = items
        .iter()
        .map(|item| {
            let detail = item_detail(item);
            UnicodeWidthStr::width(item.display.as_str())
                + UnicodeWidthStr::width(detail.as_str())
                + if detail.is_empty() { 0 } else { 2 }
        })
        .chain(std::iter::once(
            UnicodeWidthStr::width(hint)
                .saturating_add(1)
                .saturating_add(UnicodeWidthStr::width(footer)),
        ))
        .max()
        .unwrap_or(1)
        .saturating_add(4);
    desired_width.min(usize::from(terminal_width.saturating_sub(2))) as u16
}

#[cfg(test)]
mod tests {
    use super::overlay_width;
    use sopra_completion::{CompletionItem, CompletionKind};

    #[test]
    fn width_follows_content_until_the_terminal_edge() {
        let items = vec![CompletionItem::new(
            "a-very-long-command-name",
            "a description that needs room",
            "a-very-long-command-name",
            CompletionKind::Generic,
        )];
        assert_eq!(overlay_width(&items, "", "", 120), 59);
        assert_eq!(overlay_width(&items, "", "", 32), 30);
    }
}
