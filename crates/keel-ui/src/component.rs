use ratatui::{buffer::Buffer, layout::Rect};

use crate::geometry::{Constraints, Size};

pub trait Component {
    fn measure(&self, constraints: Constraints) -> Size;
    fn render(&self, area: Rect, buffer: &mut Buffer);
}

pub struct Scene {
    root: Box<dyn Component>,
}

impl Scene {
    pub fn new(root: impl Component + 'static) -> Self {
        Self {
            root: Box::new(root),
        }
    }

    pub fn measure(&self, constraints: Constraints) -> Size {
        self.root.measure(constraints)
    }

    pub fn render(&self, area: Rect, buffer: &mut Buffer) {
        self.root.render(area, buffer);
    }
}
