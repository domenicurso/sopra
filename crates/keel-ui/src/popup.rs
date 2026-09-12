use ratatui::{buffer::Buffer, layout::Rect};
use unicode_width::UnicodeWidthStr;

use crate::{
    component::Component,
    geometry::{Constraints, Size},
};

pub const MAX_VISIBLE_ITEMS: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopupItem {
    pub label: String,
    pub detail: String,
    pub match_indices: Vec<usize>,
}

impl PopupItem {
    pub fn new(label: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            detail: detail.into(),
            match_indices: Vec::new(),
        }
    }

    pub fn with_match_indices(mut self, indices: impl Into<Vec<usize>>) -> Self {
        self.match_indices = indices.into();
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PopupPlacement {
    #[default]
    Below,
    Above,
}

pub struct SuggestionPopup {
    pub(crate) footer: String,
    pub(crate) items: Vec<PopupItem>,
    pub(crate) selected: Option<usize>,
    pub(crate) query: String,
    pub(crate) anchor_column: u16,
    pub(crate) placement: PopupPlacement,
    pub(crate) viewport_start: usize,
    pub(crate) max_visible_items: usize,
}

impl SuggestionPopup {
    pub fn new(footer: impl Into<String>, items: Vec<PopupItem>, selected: Option<usize>) -> Self {
        Self {
            footer: footer.into(),
            items,
            selected,
            query: String::new(),
            anchor_column: 0,
            placement: PopupPlacement::Below,
            viewport_start: 0,
            max_visible_items: MAX_VISIBLE_ITEMS,
        }
    }

    pub fn query(mut self, query: impl Into<String>) -> Self {
        self.query = query.into();
        self
    }

    pub const fn anchor_column(mut self, anchor_column: u16) -> Self {
        self.anchor_column = anchor_column;
        self
    }

    pub const fn placement(mut self, placement: PopupPlacement) -> Self {
        self.placement = placement;
        self
    }

    pub const fn viewport(mut self, start: usize, max_visible_items: usize) -> Self {
        self.viewport_start = start;
        self.max_visible_items = if max_visible_items == 0 {
            1
        } else {
            max_visible_items
        };
        self
    }
}

impl Component for SuggestionPopup {
    fn measure(&self, constraints: Constraints) -> Size {
        let content_width = self
            .items
            .iter()
            .map(|item| {
                item.label.width()
                    + if item.detail.is_empty() {
                        0
                    } else {
                        2 + item.detail.width()
                    }
            })
            .chain(std::iter::once(self.footer.width()))
            .max()
            .unwrap_or(1);
        let visible_items = self.items.len().min(self.max_visible_items);
        Size::new(
            (content_width as u16)
                .saturating_add(4)
                .min(constraints.max_width),
            (visible_items as u16)
                .saturating_add(2)
                .min(constraints.max_height),
        )
    }

    fn render(&self, area: Rect, buffer: &mut Buffer) {
        crate::popup_render::render_popup(self, area, buffer);
    }
}
