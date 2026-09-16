use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
};
use unicode_width::UnicodeWidthStr;

use crate::completion::CompletionItem;

use super::{
    Element, MAX_OVERLAY_WIDTH,
    canvas::{Canvas, TextRun},
    overlay_items::item_detail,
};

pub(super) struct OverlayElement {
    pub(super) area: Rect,
    pub(super) items: Vec<CompletionItem>,
    pub(super) selected: usize,
    pub(super) viewport_start: usize,
    pub(super) query: String,
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
        paint_footer(canvas, self.area, &self.footer);
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
                start + offset == self.selected,
                &self.query,
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

fn viewport_start(total: usize, selected: usize, requested: usize, visible: usize) -> usize {
    let maximum = total.saturating_sub(visible);
    requested
        .max(selected.saturating_add(1).saturating_sub(visible))
        .min(maximum)
}

fn paint_footer(canvas: &mut Canvas, area: Rect, footer: &str) {
    let width = UnicodeWidthStr::width(footer) as u16;
    canvas.text(TextRun {
        position: (
            area.right().saturating_sub(1).saturating_sub(width),
            area.bottom().saturating_sub(1),
        ),
        text: footer,
        style: Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::DIM),
        max_width: width,
    });
}

pub(super) fn overlay_width(items: &[CompletionItem], footer: &str, terminal_width: u16) -> u16 {
    if terminal_width <= 2 {
        return terminal_width.max(1);
    }
    let content_width = items
        .iter()
        .map(|item| {
            let detail = item_detail(item);
            UnicodeWidthStr::width(item.display.as_str())
                + UnicodeWidthStr::width(detail.as_str())
                + if detail.is_empty() { 0 } else { 2 }
        })
        .chain(std::iter::once(UnicodeWidthStr::width(footer)))
        .max()
        .unwrap_or(1)
        .saturating_add(4) as u16;
    content_width
        .min(MAX_OVERLAY_WIDTH)
        .min(terminal_width.saturating_sub(2))
}
