use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Widget},
};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Size {
    pub width: u16,
    pub height: u16,
}

pub trait Component {
    fn measure(&self, max_width: u16) -> Size;
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

    pub fn measure(&self, max_width: u16) -> Size {
        self.root.measure(max_width)
    }

    pub fn render(&self, area: Rect, buffer: &mut Buffer) {
        self.root.render(area, buffer);
    }
}

pub struct Text {
    line: Line<'static>,
}

impl Text {
    pub fn styled(text: impl Into<String>, style: Style) -> Self {
        Self {
            line: Line::from(Span::styled(text.into(), style)),
        }
    }
}

impl Component for Text {
    fn measure(&self, max_width: u16) -> Size {
        let width = self
            .line
            .spans
            .iter()
            .map(|span| UnicodeWidthStr::width(span.content.as_ref()))
            .sum::<usize>()
            .min(max_width as usize) as u16;
        Size { width, height: 1 }
    }

    fn render(&self, area: Rect, buffer: &mut Buffer) {
        Paragraph::new(self.line.clone()).render(area, buffer);
    }
}

pub struct Badge {
    label: String,
    text_style: Style,
    border_style: Style,
}

impl Badge {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            text_style: Style::default()
                .fg(Color::Rgb(125, 211, 252))
                .bg(Color::Rgb(15, 23, 42))
                .add_modifier(Modifier::BOLD),
            border_style: Style::default().fg(Color::Rgb(45, 212, 191)),
        }
    }
}

impl Component for Badge {
    fn measure(&self, max_width: u16) -> Size {
        let content_width = UnicodeWidthStr::width(self.label.as_str()) as u16;
        Size {
            width: content_width.saturating_add(4).min(max_width.max(1)),
            height: 1,
        }
    }

    fn render(&self, area: Rect, buffer: &mut Buffer) {
        Paragraph::new(Line::from(Span::styled(
            self.label.clone(),
            self.text_style,
        )))
        .style(self.text_style)
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .border_style(self.border_style)
                .padding(Padding::horizontal(1)),
        )
        .render(area, buffer);
    }
}

pub fn badge_scene(label: impl Into<String>) -> Scene {
    Scene::new(Badge::new(label))
}
