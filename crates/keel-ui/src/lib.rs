use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph as RatatuiParagraph, Widget, Wrap},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Size {
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Constraints {
    pub max_width: u16,
    pub max_height: u16,
}

impl Constraints {
    pub const fn new(max_width: u16, max_height: u16) -> Self {
        Self {
            max_width,
            max_height,
        }
    }
}

pub trait Component {
    fn measure(&self, constraints: Constraints) -> Size;
    fn render(&self, area: Rect, buffer: &mut Buffer);
}

/// Adapts a cloneable ratatui `Widget` to the Keel component tree.
///
/// This keeps the composition API small while allowing callers to use
/// ratatui widgets directly whenever a built-in component is not enough.
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
        Size {
            width: self.size.width.min(constraints.max_width),
            height: self.size.height.min(constraints.max_height),
        }
    }

    fn render(&self, area: Rect, buffer: &mut Buffer) {
        self.widget.clone().render(area, buffer);
    }
}

pub fn ratatui_component<W: Widget + Clone>(widget: W, size: Size) -> RatatuiComponent<W> {
    RatatuiComponent::new(widget, size)
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
        self.root
            .measure(Constraints::new(max_width.max(1), u16::MAX))
    }

    pub fn render(&self, area: Rect, buffer: &mut Buffer) {
        self.root.render(area, buffer);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub accent: Style,
    pub value: Style,
    pub muted: Style,
    pub surface: Style,
    pub border: Style,
    pub cursor: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            accent: Style::default()
                .fg(Color::Rgb(125, 211, 252))
                .add_modifier(Modifier::BOLD),
            value: Style::default()
                .fg(Color::Rgb(250, 204, 21))
                .add_modifier(Modifier::BOLD),
            muted: Style::default().fg(Color::Rgb(100, 116, 139)),
            surface: Style::default().bg(Color::Rgb(15, 23, 42)),
            border: Style::default().fg(Color::Rgb(45, 212, 191)),
            cursor: Style::default()
                .fg(Color::Rgb(15, 23, 42))
                .bg(Color::Rgb(125, 211, 252)),
        }
    }
}

impl Theme {
    pub fn named(name: &str) -> Self {
        match name.trim().to_ascii_lowercase().as_str() {
            "mono" | "monochrome" => Self {
                accent: Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
                value: Style::default().fg(Color::White),
                muted: Style::default().fg(Color::DarkGray),
                surface: Style::default(),
                border: Style::default().fg(Color::Gray),
                cursor: Style::default().fg(Color::Black).bg(Color::White),
            },
            "amber" => Self {
                accent: Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
                value: Style::default()
                    .fg(Color::Rgb(251, 146, 60))
                    .add_modifier(Modifier::BOLD),
                muted: Style::default().fg(Color::DarkGray),
                surface: Style::default().bg(Color::Rgb(28, 25, 23)),
                border: Style::default().fg(Color::LightYellow),
                cursor: Style::default().fg(Color::Black).bg(Color::LightYellow),
            },
            _ => Self::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleToken {
    Accent,
    Value,
    Muted,
    Surface,
    Border,
    Cursor,
}

impl StyleToken {
    pub fn resolve(self, theme: Theme) -> Style {
        match self {
            Self::Accent => theme.accent,
            Self::Value => theme.value,
            Self::Muted => theme.muted,
            Self::Surface => theme.surface,
            Self::Border => theme.border,
            Self::Cursor => theme.cursor,
        }
    }
}

pub struct Text {
    lines: Vec<Line<'static>>,
    wrap: bool,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            lines: vec![Line::from(text.into())],
            wrap: false,
        }
    }

    pub fn styled(text: impl Into<String>, style: Style) -> Self {
        Self {
            lines: vec![Line::from(Span::styled(text.into(), style))],
            wrap: false,
        }
    }

    pub fn token(text: impl Into<String>, token: StyleToken, theme: Theme) -> Self {
        Self::styled(text, token.resolve(theme))
    }

    pub fn lines(lines: Vec<Line<'static>>) -> Self {
        Self { lines, wrap: false }
    }

    pub fn with_wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }
}

impl Component for Text {
    fn measure(&self, constraints: Constraints) -> Size {
        let width = self.lines.iter().map(line_width).max().unwrap_or_default();
        let height = if self.wrap {
            wrapped_height(&self.lines, constraints.max_width)
        } else {
            self.lines.len().max(1).min(u16::MAX as usize) as u16
        };
        Size {
            width: width.min(constraints.max_width as usize) as u16,
            height: height.min(constraints.max_height),
        }
    }

    fn render(&self, area: Rect, buffer: &mut Buffer) {
        let width = if self.wrap {
            area.width
        } else {
            self.lines
                .iter()
                .map(line_width)
                .max()
                .unwrap_or(1)
                .min(area.width as usize)
                .max(1) as u16
        };
        let render_area = Rect::new(area.x, area.y, width, area.height);
        let mut paragraph = RatatuiParagraph::new(self.lines.clone());
        if self.wrap {
            paragraph = paragraph.wrap(Wrap { trim: false });
        }
        paragraph.render(render_area, buffer);
    }
}

pub struct Paragraph {
    lines: Vec<Line<'static>>,
    style: Style,
    wrap: bool,
    border: Option<Style>,
    padding: u16,
}

impl Paragraph {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            lines: {
                let text: String = text.into();
                text.split('\n')
                    .map(|line| Line::from(line.to_string()))
                    .collect()
            },
            style: Style::default(),
            wrap: true,
            border: None,
            padding: 0,
        }
    }

    pub fn from_lines(lines: Vec<Line<'static>>) -> Self {
        Self {
            lines,
            style: Style::default(),
            wrap: true,
            border: None,
            padding: 0,
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn token(mut self, token: StyleToken, theme: Theme) -> Self {
        self.style = token.resolve(theme);
        self
    }

    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn bordered(mut self, style: Style) -> Self {
        self.border = Some(style);
        self
    }

    pub fn padding(mut self, padding: u16) -> Self {
        self.padding = padding;
        self
    }

    fn chrome_width(&self) -> u16 {
        let border = u16::from(self.border.is_some()) * 2;
        border.saturating_add(self.padding.saturating_mul(2))
    }

    fn chrome_height(&self) -> u16 {
        self.chrome_width()
    }
}

impl Component for Paragraph {
    fn measure(&self, constraints: Constraints) -> Size {
        let chrome_width = self.chrome_width();
        let chrome_height = self.chrome_height();
        let content_width = constraints.max_width.saturating_sub(chrome_width).max(1);
        let content_height = if self.wrap {
            wrapped_height(&self.lines, content_width)
        } else {
            self.lines.len().max(1).min(u16::MAX as usize) as u16
        };
        Size {
            width: self
                .lines
                .iter()
                .map(line_width)
                .max()
                .unwrap_or_default()
                .saturating_add(chrome_width as usize)
                .min(constraints.max_width as usize) as u16,
            height: content_height
                .saturating_add(chrome_height)
                .min(constraints.max_height),
        }
    }

    fn render(&self, area: Rect, buffer: &mut Buffer) {
        let inner = if let Some(border_style) = self.border {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .padding(Padding::uniform(self.padding))
                .style(self.style);
            let inner = block.inner(area);
            block.render(area, buffer);
            inner
        } else {
            Rect::new(
                area.x.saturating_add(self.padding),
                area.y.saturating_add(self.padding),
                area.width.saturating_sub(self.padding.saturating_mul(2)),
                area.height.saturating_sub(self.padding.saturating_mul(2)),
            )
        };

        let mut paragraph = RatatuiParagraph::new(self.lines.clone()).style(self.style);
        if self.wrap {
            paragraph = paragraph.wrap(Wrap { trim: false });
        }
        paragraph.render(inner, buffer);
    }
}

pub struct Panel {
    child: Box<dyn Component>,
    style: Style,
    border: Option<Style>,
    borders: Borders,
    padding: Padding,
}

impl Panel {
    pub fn new(child: impl Component + 'static) -> Self {
        Self {
            child: Box::new(child),
            style: Style::default(),
            border: None,
            borders: Borders::ALL,
            padding: Padding::ZERO,
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn bordered(mut self, style: Style) -> Self {
        self.border = Some(style);
        self
    }

    pub fn borders(mut self, borders: Borders) -> Self {
        self.borders = borders;
        self
    }

    pub fn padding(mut self, padding: u16) -> Self {
        self.padding = Padding::uniform(padding);
        self
    }

    pub fn padding_horizontal(mut self, padding: u16) -> Self {
        self.padding.left = padding;
        self.padding.right = padding;
        self
    }

    pub fn padding_vertical(mut self, padding: u16) -> Self {
        self.padding.top = padding;
        self.padding.bottom = padding;
        self
    }

    fn chrome_width(&self) -> u16 {
        let border = u16::from(self.borders.contains(Borders::LEFT))
            + u16::from(self.borders.contains(Borders::RIGHT));
        border
            .saturating_add(self.padding.left)
            .saturating_add(self.padding.right)
    }

    fn chrome_height(&self) -> u16 {
        let border = u16::from(self.borders.contains(Borders::TOP))
            + u16::from(self.borders.contains(Borders::BOTTOM));
        border
            .saturating_add(self.padding.top)
            .saturating_add(self.padding.bottom)
    }
}

impl Component for Panel {
    fn measure(&self, constraints: Constraints) -> Size {
        let chrome_width = self.chrome_width();
        let chrome_height = self.chrome_height();
        let content = self.child.measure(Constraints::new(
            constraints.max_width.saturating_sub(chrome_width).max(1),
            constraints.max_height.saturating_sub(chrome_height).max(1),
        ));
        Size {
            width: content
                .width
                .saturating_add(chrome_width)
                .min(constraints.max_width),
            height: content
                .height
                .saturating_add(chrome_height)
                .min(constraints.max_height),
        }
    }

    fn render(&self, area: Rect, buffer: &mut Buffer) {
        let inner = if let Some(border_style) = self.border {
            let block = Block::default()
                .borders(self.borders)
                .border_style(border_style)
                .padding(self.padding)
                .style(self.style);
            let inner = block.inner(area);
            block.render(area, buffer);
            inner
        } else {
            Rect::new(
                area.x.saturating_add(self.padding.left),
                area.y.saturating_add(self.padding.top),
                area.width
                    .saturating_sub(self.padding.left.saturating_add(self.padding.right)),
                area.height
                    .saturating_sub(self.padding.top.saturating_add(self.padding.bottom)),
            )
        };
        self.child.render(inner, buffer);
    }
}

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

    pub fn gap(mut self, gap: u16) -> Self {
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
        let sizes = self
            .children
            .iter()
            .map(|child| child.measure(constraints))
            .collect::<Vec<_>>();
        Size {
            width: sizes.iter().map(|size| size.width).max().unwrap_or(0),
            height: sizes
                .iter()
                .map(|size| size.height)
                .fold(0, u16::saturating_add)
                .saturating_add(
                    self.gap
                        .saturating_mul(self.children.len().saturating_sub(1) as u16),
                )
                .min(constraints.max_height),
        }
    }

    fn render(&self, area: Rect, buffer: &mut Buffer) {
        let mut constraints = Vec::with_capacity(self.children.len() * 2);
        for (index, child) in self.children.iter().enumerate() {
            let size = child.measure(Constraints::new(area.width, area.height));
            constraints.push(Constraint::Length(size.height));
            if index + 1 < self.children.len() {
                constraints.push(Constraint::Length(self.gap));
            }
        }
        let regions = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);
        let mut region_index = 0;
        for child in &self.children {
            child.render(regions[region_index], buffer);
            region_index += 1;
            if region_index < regions.len() && region_index < self.children.len() * 2 {
                region_index += 1;
            }
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

    pub fn gap(mut self, gap: u16) -> Self {
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
        let sizes = self
            .children
            .iter()
            .map(|child| child.measure(constraints))
            .collect::<Vec<_>>();
        Size {
            width: sizes
                .iter()
                .map(|size| size.width)
                .fold(0, u16::saturating_add)
                .saturating_add(
                    self.gap
                        .saturating_mul(self.children.len().saturating_sub(1) as u16),
                )
                .min(constraints.max_width),
            height: sizes.iter().map(|size| size.height).max().unwrap_or(0),
        }
    }

    fn render(&self, area: Rect, buffer: &mut Buffer) {
        let mut constraints = Vec::with_capacity(self.children.len() * 2);
        for (index, child) in self.children.iter().enumerate() {
            let size = child.measure(Constraints::new(area.width, area.height));
            constraints.push(Constraint::Length(size.width));
            if index + 1 < self.children.len() {
                constraints.push(Constraint::Length(self.gap));
            }
        }
        let regions = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(area);
        let mut region_index = 0;
        for child in &self.children {
            child.render(regions[region_index], buffer);
            region_index += 1;
            if region_index < regions.len() && region_index < self.children.len() * 2 {
                region_index += 1;
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalAlign {
    Left,
    Center,
    Right,
}

pub struct Align {
    child: Box<dyn Component>,
    horizontal: HorizontalAlign,
}

impl Align {
    pub fn left(child: impl Component + 'static) -> Self {
        Self {
            child: Box::new(child),
            horizontal: HorizontalAlign::Left,
        }
    }

    pub fn center(child: impl Component + 'static) -> Self {
        Self {
            child: Box::new(child),
            horizontal: HorizontalAlign::Center,
        }
    }

    pub fn right(child: impl Component + 'static) -> Self {
        Self {
            child: Box::new(child),
            horizontal: HorizontalAlign::Right,
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
        let width = size.width.min(area.width);
        let x = match self.horizontal {
            HorizontalAlign::Left => area.x,
            HorizontalAlign::Center => area.x + area.width.saturating_sub(width) / 2,
            HorizontalAlign::Right => area.x + area.width.saturating_sub(width),
        };
        self.child
            .render(Rect::new(x, area.y, width, area.height), buffer);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpacerAxis {
    Horizontal,
    Vertical,
}

pub struct Spacer {
    axis: SpacerAxis,
    amount: u16,
}

impl Spacer {
    pub fn horizontal(amount: u16) -> Self {
        Self {
            axis: SpacerAxis::Horizontal,
            amount,
        }
    }

    pub fn vertical(amount: u16) -> Self {
        Self {
            axis: SpacerAxis::Vertical,
            amount,
        }
    }
}

impl Component for Spacer {
    fn measure(&self, constraints: Constraints) -> Size {
        match self.axis {
            SpacerAxis::Horizontal => Size {
                width: self.amount.min(constraints.max_width),
                height: 1.min(constraints.max_height),
            },
            SpacerAxis::Vertical => Size {
                width: 1.min(constraints.max_width),
                height: self.amount.min(constraints.max_height),
            },
        }
    }

    fn render(&self, _area: Rect, _buffer: &mut Buffer) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleAxis {
    Horizontal,
    Vertical,
}

pub struct Rule {
    axis: RuleAxis,
    style: Style,
    symbol: &'static str,
}

impl Rule {
    pub fn horizontal(style: Style) -> Self {
        Self {
            axis: RuleAxis::Horizontal,
            style,
            symbol: "─",
        }
    }

    pub fn vertical(style: Style) -> Self {
        Self {
            axis: RuleAxis::Vertical,
            style,
            symbol: "│",
        }
    }

    pub fn symbol(mut self, symbol: &'static str) -> Self {
        self.symbol = symbol;
        self
    }
}

impl Component for Rule {
    fn measure(&self, constraints: Constraints) -> Size {
        match self.axis {
            RuleAxis::Horizontal => Size {
                width: constraints.max_width,
                height: 1.min(constraints.max_height),
            },
            RuleAxis::Vertical => Size {
                width: 1.min(constraints.max_width),
                height: constraints.max_height,
            },
        }
    }

    fn render(&self, area: Rect, buffer: &mut Buffer) {
        match self.axis {
            RuleAxis::Horizontal => {
                for x in area.left()..area.right() {
                    buffer[(x, area.top())]
                        .set_symbol(self.symbol)
                        .set_style(self.style);
                }
            }
            RuleAxis::Vertical => {
                for y in area.top()..area.bottom() {
                    buffer[(area.left(), y)]
                        .set_symbol(self.symbol)
                        .set_style(self.style);
                }
            }
        }
    }
}

pub struct InputLine {
    prefix: Line<'static>,
    text: String,
    cursor: usize,
    text_style: Style,
    cursor_style: Style,
}

impl InputLine {
    pub fn new(prefix: impl Into<String>, text: impl Into<String>, cursor: usize) -> Self {
        Self {
            prefix: Line::from(prefix.into()),
            text: text.into(),
            cursor,
            text_style: Style::default(),
            cursor_style: Theme::default().cursor,
        }
    }

    pub fn styles(mut self, text_style: Style, cursor_style: Style) -> Self {
        self.text_style = text_style;
        self.cursor_style = cursor_style;
        self
    }
}

impl Component for InputLine {
    fn measure(&self, constraints: Constraints) -> Size {
        let width = line_width(&self.prefix)
            .saturating_add(self.text.width())
            .min(constraints.max_width as usize) as u16;
        let height = wrapped_height(&[self.input_line()], constraints.max_width);
        Size {
            width,
            height: height.min(constraints.max_height),
        }
    }

    fn render(&self, area: Rect, buffer: &mut Buffer) {
        let line = self.input_line();
        RatatuiParagraph::new(line)
            .wrap(Wrap { trim: false })
            .render(area, buffer);

        if area.width == 0 || area.height == 0 {
            return;
        }
        let prefix_width = line_width(&self.prefix);
        let text_width = self
            .text
            .graphemes(true)
            .take(self.cursor.min(self.text.graphemes(true).count()))
            .map(UnicodeWidthStr::width)
            .sum::<usize>();
        let offset = prefix_width.saturating_add(text_width);
        let x = (offset % area.width as usize) as u16;
        let y = (offset / area.width as usize) as u16;
        if y < area.height {
            buffer[(area.x + x, area.y + y)].set_style(self.cursor_style);
        }
    }
}

impl InputLine {
    fn input_line(&self) -> Line<'static> {
        let mut spans = self.prefix.spans.clone();
        spans.push(Span::styled(self.text.clone(), self.text_style));
        Line::from(spans)
    }
}

pub fn badge_component(label: impl Into<String>, theme: Theme) -> Panel {
    let label = label.into();
    Panel::new(
        Paragraph::from_lines(vec![Line::from(Span::styled(label, theme.accent))]).wrap(false),
    )
    .style(theme.surface)
    .bordered(theme.border)
    .borders(Borders::LEFT | Borders::RIGHT)
    .padding_horizontal(1)
}

pub fn badge_scene(label: impl Into<String>) -> Scene {
    Scene::new(Align::right(badge_component(label, Theme::default())))
}

fn line_width(line: &Line<'_>) -> usize {
    line.spans
        .iter()
        .map(|span| UnicodeWidthStr::width(span.content.as_ref()))
        .sum()
}

fn wrapped_height(lines: &[Line<'_>], width: u16) -> u16 {
    let width = width.max(1) as usize;
    lines
        .iter()
        .map(|line| line_width(line).max(1).div_ceil(width))
        .fold(0usize, usize::saturating_add)
        .max(1)
        .min(u16::MAX as usize) as u16
}

#[cfg(test)]
mod tests {
    use ratatui::{
        buffer::Buffer,
        layout::Rect,
        style::Color,
        widgets::{Block, Borders},
    };

    use super::{
        Align, Component, Constraints, InputLine, Paragraph, Rule, Scene, Size, StyleToken, Text,
        Theme, badge_component, ratatui_component,
    };

    #[test]
    fn paragraph_measures_multiline_wrapped_content() {
        let paragraph = Paragraph::new("hello\nworld");
        assert_eq!(
            paragraph.measure(Constraints::new(20, 20)),
            Size {
                width: 5,
                height: 2
            }
        );
    }

    #[test]
    fn right_aligned_component_uses_the_available_width() {
        let scene = Scene::new(Align::right(Text::token(
            "Keel",
            StyleToken::Accent,
            Theme::default(),
        )));
        let area = Rect::new(0, 0, 10, 1);
        let mut buffer = Buffer::empty(area);
        scene.render(area, &mut buffer);

        assert_eq!(buffer[(6, 0)].symbol(), "K");
        assert_eq!(buffer[(6, 0)].fg, Color::Rgb(125, 211, 252));
    }

    #[test]
    fn input_cursor_is_painted_at_the_wrapped_grapheme_position() {
        let input = InputLine::new("> ", "ab界", 2);
        let area = Rect::new(0, 0, 4, 2);
        let mut buffer = Buffer::empty(area);
        input.render(area, &mut buffer);

        assert_eq!(buffer[(0, 1)].bg, Theme::default().cursor.bg.unwrap());
    }

    #[test]
    fn badge_component_stays_one_row_tall() {
        let size = badge_component("Keel", Theme::default()).measure(Constraints::new(80, 20));
        assert_eq!(size.height, 1);
    }

    #[test]
    fn arbitrary_ratatui_widget_can_join_a_scene() {
        let scene = Scene::new(ratatui_component(
            Block::default().borders(Borders::ALL),
            Size {
                width: 4,
                height: 3,
            },
        ));
        let area = Rect::new(0, 0, 4, 3);
        let mut buffer = Buffer::empty(area);
        scene.render(area, &mut buffer);

        assert_eq!(buffer[(0, 0)].symbol(), "┌");
        assert_eq!(scene.measure(80).height, 3);
    }

    #[test]
    fn horizontal_rule_expands_to_the_available_width() {
        let rule = Rule::horizontal(Theme::default().border);
        let area = Rect::new(0, 0, 6, 1);
        let mut buffer = Buffer::empty(area);

        rule.render(area, &mut buffer);

        assert_eq!(
            rule.measure(Constraints::new(6, 4)),
            Size {
                width: 6,
                height: 1
            }
        );
        assert_eq!(buffer[(5, 0)].symbol(), "─");
    }
}
