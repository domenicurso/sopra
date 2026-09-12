use ratatui::{buffer::Buffer, layout::Rect};

use crate::{
    component::Component,
    geometry::{Constraints, Size},
};

pub struct Column {
    children: Vec<Box<dyn Component>>,
    gap: u16,
}

impl Column {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            gap: 0,
        }
    }

    pub fn child(mut self, child: impl Component + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    pub const fn gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }
}

impl Default for Column {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for Column {
    fn measure(&self, constraints: Constraints) -> Size {
        let mut width: u16 = 0;
        let mut height: u16 = 0;
        for (index, child) in self.children.iter().enumerate() {
            let size = child.measure(constraints);
            width = width.max(size.width);
            height = height.saturating_add(size.height);
            if index > 0 {
                height = height.saturating_add(self.gap);
            }
        }
        Size::new(
            width.min(constraints.max_width),
            height.min(constraints.max_height),
        )
    }

    fn render(&self, area: Rect, buffer: &mut Buffer) {
        let mut y = area.y;
        for (index, child) in self.children.iter().enumerate() {
            if index > 0 {
                y = y.saturating_add(self.gap);
            }
            if y >= area.y.saturating_add(area.height) {
                break;
            }
            let remaining = area.y.saturating_add(area.height).saturating_sub(y);
            let size = child.measure(Constraints::new(area.width, remaining));
            let child_area = Rect::new(area.x, y, area.width, size.height.min(remaining));
            child.render(child_area, buffer);
            y = y.saturating_add(child_area.height);
        }
    }
}

pub struct Row {
    children: Vec<Box<dyn Component>>,
    gap: u16,
}

impl Row {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            gap: 0,
        }
    }

    pub fn child(mut self, child: impl Component + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    pub const fn gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }
}

impl Default for Row {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for Row {
    fn measure(&self, constraints: Constraints) -> Size {
        let mut width: u16 = 0;
        let mut height: u16 = 0;
        for (index, child) in self.children.iter().enumerate() {
            let size = child.measure(constraints);
            width = width.saturating_add(size.width);
            height = height.max(size.height);
            if index > 0 {
                width = width.saturating_add(self.gap);
            }
        }
        Size::new(
            width.min(constraints.max_width),
            height.min(constraints.max_height),
        )
    }

    fn render(&self, area: Rect, buffer: &mut Buffer) {
        let mut x = area.x;
        for (index, child) in self.children.iter().enumerate() {
            if index > 0 {
                x = x.saturating_add(self.gap);
            }
            if x >= area.x.saturating_add(area.width) {
                break;
            }
            let remaining = area.x.saturating_add(area.width).saturating_sub(x);
            let size = child.measure(Constraints::new(remaining, area.height));
            let child_area = Rect::new(x, area.y, size.width.min(remaining), area.height);
            child.render(child_area, buffer);
            x = x.saturating_add(child_area.width);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalAlignment {
    Left,
    Center,
    Right,
}

pub struct Align {
    child: Box<dyn Component>,
    alignment: HorizontalAlignment,
}

impl Align {
    pub fn new(child: impl Component + 'static, alignment: HorizontalAlignment) -> Self {
        Self {
            child: Box::new(child),
            alignment,
        }
    }
}

impl Component for Align {
    fn measure(&self, constraints: Constraints) -> Size {
        self.child.measure(constraints)
    }

    fn render(&self, area: Rect, buffer: &mut Buffer) {
        let size = self
            .child
            .measure(Constraints::new(area.width, area.height));
        let x = match self.alignment {
            HorizontalAlignment::Left => area.x,
            HorizontalAlignment::Center => area
                .x
                .saturating_add(area.width.saturating_sub(size.width) / 2),
            HorizontalAlignment::Right => {
                area.x.saturating_add(area.width.saturating_sub(size.width))
            }
        };
        self.child
            .render(Rect::new(x, area.y, size.width, area.height), buffer);
    }
}
