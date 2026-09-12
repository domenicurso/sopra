use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use crate::{
    component::Component,
    geometry::{Constraints, Size},
};

pub struct RatatuiComponent<W> {
    widget: W,
    size: Size,
}

impl<W> RatatuiComponent<W> {
    pub fn new(widget: W, size: Size) -> Self {
        Self { widget, size }
    }
}

impl<W: Widget + Clone> Component for RatatuiComponent<W> {
    fn measure(&self, constraints: Constraints) -> Size {
        Size::new(
            self.size.width.min(constraints.max_width),
            self.size.height.min(constraints.max_height),
        )
    }

    fn render(&self, area: Rect, buffer: &mut Buffer) {
        self.widget.clone().render(area, buffer);
    }
}

pub fn ratatui_component<W: Widget + Clone>(widget: W, size: Size) -> RatatuiComponent<W> {
    RatatuiComponent::new(widget, size)
}
