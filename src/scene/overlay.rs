use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
};
use unicode_width::UnicodeWidthStr;

use crate::completion::CompletionItem;

use super::{
    Element, MAX_OVERLAY_WIDTH, OVERLAY_FOOTER,
    canvas::{Canvas, TextRun},
};

pub(super) struct OverlayElement {
    pub(super) area: Rect,
    pub(super) items: Vec<CompletionItem>,
    pub(super) selected: usize,
    pub(super) connector_column: u16,
    pub(super) connector: &'static str,
}

impl Element for OverlayElement {
    fn paint(&self, canvas: &mut Canvas) {
        if self.area.width < 8 || self.area.height < 3 {
            return;
        }
        let border = Style::default().fg(Color::Cyan);
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
        let start = viewport_start(self.items.len(), self.selected, visible);
        for (offset, item) in self.items[start..].iter().take(visible).enumerate() {
            let row = inner.top().saturating_add(offset as u16);
            super::overlay_items::paint_item(
                canvas,
                inner,
                row,
                item,
                start + offset == self.selected,
            );
        }
        if self.items.len() > visible {
            super::overlay_items::paint_scrollbar(canvas, inner, start, visible, self.items.len());
        }
    }
}

fn viewport_start(total: usize, selected: usize, visible: usize) -> usize {
    selected
        .saturating_sub(visible.saturating_sub(1))
        .min(total.saturating_sub(visible))
}

fn paint_title(canvas: &mut Canvas, area: Rect) {
    canvas.text(TextRun {
        position: (area.left().saturating_add(2), area.top()),
        text: " Keel ",
        style: Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
        max_width: area.width.saturating_sub(4),
    });
}

fn paint_footer(canvas: &mut Canvas, area: Rect) {
    canvas.text(TextRun {
        position: (
            area.left().saturating_add(2),
            area.bottom().saturating_sub(1),
        ),
        text: OVERLAY_FOOTER,
        style: Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM),
        max_width: area.width.saturating_sub(4),
    });
}

pub(super) fn overlay_width(items: &[CompletionItem], terminal_width: u16) -> u16 {
    if terminal_width <= 2 {
        return terminal_width.max(1);
    }
    let content_width = items
        .iter()
        .map(|item| {
            UnicodeWidthStr::width(item.display.as_str())
                + if item.description.is_none() && item.location.is_none() {
                    0
                } else {
                    UnicodeWidthStr::width(item_detail(item).as_str()) + 2
                }
                + 3
        })
        .chain(std::iter::once(UnicodeWidthStr::width(OVERLAY_FOOTER)))
        .max()
        .unwrap_or(1)
        .saturating_add(4) as u16;
    content_width
        .min(MAX_OVERLAY_WIDTH)
        .min(terminal_width.saturating_sub(2))
}

fn item_detail(item: &CompletionItem) -> String {
    item.description
        .clone()
        .or_else(|| {
            item.location
                .as_ref()
                .map(|path| path.display().to_string())
        })
        .unwrap_or_default()
}
