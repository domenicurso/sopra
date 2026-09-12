use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph as RatatuiParagraph, Widget, Wrap},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Size {
    pub width: u16,
    pub height: u16,
}

impl Size {
    pub const fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub accent: Style,
    pub value: Style,
    pub muted: Style,
    pub surface: Style,
    pub border: Style,
    pub selection: Style,
    pub cursor: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self::flyline()
    }
}

impl Theme {
    pub const fn flyline() -> Self {
        Self {
            accent: Style::new()
                .fg(Color::Rgb(48, 197, 128))
                .add_modifier(Modifier::BOLD),
            value: Style::new()
                .fg(Color::Rgb(214, 214, 214))
                .add_modifier(Modifier::BOLD),
            muted: Style::new().fg(Color::Rgb(105, 105, 105)),
            surface: Style::new().bg(Color::Rgb(23, 23, 23)),
            border: Style::new().fg(Color::Rgb(105, 105, 105)),
            selection: Style::new()
                .fg(Color::Rgb(23, 23, 23))
                .bg(Color::Rgb(48, 197, 128)),
            cursor: Style::new().fg(Color::Rgb(23, 23, 23)).bg(Color::White),
        }
    }

    pub fn named(name: &str) -> Self {
        match name.trim().to_ascii_lowercase().as_str() {
            "mono" | "monochrome" => Self {
                accent: Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
                value: Style::new().fg(Color::White),
                muted: Style::new().fg(Color::DarkGray),
                surface: Style::default(),
                border: Style::new().fg(Color::Gray),
                selection: Style::new().fg(Color::Black).bg(Color::White),
                cursor: Style::new().fg(Color::Black).bg(Color::White),
            },
            "amber" => Self {
                accent: Style::new()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
                value: Style::new()
                    .fg(Color::Rgb(251, 146, 60))
                    .add_modifier(Modifier::BOLD),
                muted: Style::new().fg(Color::DarkGray),
                surface: Style::new().bg(Color::Rgb(28, 25, 23)),
                border: Style::new().fg(Color::LightYellow),
                selection: Style::new().fg(Color::Black).bg(Color::LightYellow),
                cursor: Style::new().fg(Color::Black).bg(Color::LightYellow),
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
    Selection,
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
            Self::Selection => theme.selection,
            Self::Cursor => theme.cursor,
        }
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

pub struct Spacer {
    size: Size,
    style: Style,
}

impl Spacer {
    pub const fn new(width: u16, height: u16) -> Self {
        Self {
            size: Size::new(width, height),
            style: Style::new(),
        }
    }

    pub const fn height(height: u16) -> Self {
        Self::new(0, height)
    }

    pub const fn width(width: u16) -> Self {
        Self::new(width, 0)
    }

    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Component for Spacer {
    fn measure(&self, constraints: Constraints) -> Size {
        Size::new(
            self.size.width.min(constraints.max_width),
            self.size.height.min(constraints.max_height),
        )
    }

    fn render(&self, area: Rect, buffer: &mut Buffer) {
        if self.style != Style::default() {
            buffer.set_style(area, self.style);
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
            lines: text
                .into()
                .split('\n')
                .map(|line| Line::from(line.to_string()))
                .collect(),
            wrap: false,
        }
    }

    pub fn styled(text: impl Into<String>, style: Style) -> Self {
        Self {
            lines: vec![Line::from(Span::styled(text.into(), style))],
            wrap: false,
        }
    }

    pub fn lines(lines: Vec<Line<'static>>) -> Self {
        Self { lines, wrap: false }
    }

    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }
}

impl Component for Text {
    fn measure(&self, constraints: Constraints) -> Size {
        let width = self.lines.iter().map(line_width).max().unwrap_or(0);
        let height = if self.wrap {
            self.lines
                .iter()
                .map(|line| wrapped_line_height(line, constraints.max_width))
                .sum::<usize>()
        } else {
            self.lines.len().max(1)
        };
        Size::new(
            (width as u16).min(constraints.max_width),
            (height as u16).min(constraints.max_height),
        )
    }

    fn render(&self, area: Rect, buffer: &mut Buffer) {
        let mut paragraph = RatatuiParagraph::new(self.lines.clone());
        if self.wrap {
            paragraph = paragraph.wrap(Wrap { trim: false });
        }
        paragraph.render(area, buffer);
    }
}

pub struct Paragraph {
    lines: Vec<Line<'static>>,
    style: Style,
    block: Option<Block<'static>>,
    wrap: bool,
}

impl Paragraph {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            lines: text
                .into()
                .split('\n')
                .map(|line| Line::from(line.to_string()))
                .collect(),
            style: Style::default(),
            block: None,
            wrap: true,
        }
    }

    pub fn from_lines(lines: Vec<Line<'static>>) -> Self {
        Self {
            lines,
            style: Style::default(),
            block: None,
            wrap: true,
        }
    }

    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn block(mut self, block: Block<'static>) -> Self {
        self.block = Some(block);
        self
    }

    pub const fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }
}

impl Component for Paragraph {
    fn measure(&self, constraints: Constraints) -> Size {
        let (horizontal, vertical) = self
            .block
            .as_ref()
            .map_or((0, 0), |block| block_dimensions(block, constraints));
        let content_width = constraints.max_width.saturating_sub(horizontal);
        let content_height = if self.wrap {
            self.lines
                .iter()
                .map(|line| wrapped_line_height(line, content_width))
                .sum::<usize>()
        } else {
            self.lines.len().max(1)
        } as u16;
        let content_width_used = self.lines.iter().map(line_width).max().unwrap_or(0) as u16;
        Size::new(
            content_width_used
                .saturating_add(horizontal)
                .min(constraints.max_width),
            content_height
                .saturating_add(vertical)
                .min(constraints.max_height),
        )
    }

    fn render(&self, area: Rect, buffer: &mut Buffer) {
        let mut paragraph = RatatuiParagraph::new(self.lines.clone()).style(self.style);
        if self.wrap {
            paragraph = paragraph.wrap(Wrap { trim: false });
        }
        if let Some(block) = self.block.clone() {
            paragraph = paragraph.block(block);
        }
        paragraph.render(area, buffer);
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

    pub const fn text_style(mut self, style: Style) -> Self {
        self.text_style = style;
        self
    }

    pub const fn cursor_style(mut self, style: Style) -> Self {
        self.cursor_style = style;
        self
    }
}

impl Component for InputLine {
    fn measure(&self, constraints: Constraints) -> Size {
        let text_width = self.text.width() as u16;
        Size::new(
            (line_width(&self.prefix) as u16)
                .saturating_add(text_width)
                .min(constraints.max_width),
            1.min(constraints.max_height),
        )
    }

    fn render(&self, area: Rect, buffer: &mut Buffer) {
        let graphemes = self.text.graphemes(true).collect::<Vec<_>>();
        let cursor = self.cursor.min(graphemes.len());
        let mut spans = self.prefix.spans.clone();
        spans.push(Span::styled(graphemes[..cursor].concat(), self.text_style));
        if cursor < graphemes.len() {
            spans.push(Span::styled(
                graphemes[cursor].to_string(),
                self.cursor_style,
            ));
            spans.push(Span::styled(
                graphemes[cursor + 1..].concat(),
                self.text_style,
            ));
        } else {
            spans.push(Span::styled(" ", self.cursor_style));
        }
        RatatuiParagraph::new(Line::from(spans)).render(area, buffer);
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopupItem {
    pub label: String,
    pub detail: String,
}

impl PopupItem {
    pub fn new(label: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            detail: detail.into(),
        }
    }
}

pub struct SuggestionPopup {
    title: String,
    footer: String,
    items: Vec<PopupItem>,
    selected: usize,
    theme: Theme,
}

impl SuggestionPopup {
    pub fn new(
        title: impl Into<String>,
        footer: impl Into<String>,
        items: Vec<PopupItem>,
        selected: usize,
        theme: Theme,
    ) -> Self {
        Self {
            title: title.into(),
            footer: footer.into(),
            items,
            selected,
            theme,
        }
    }
}

impl Component for SuggestionPopup {
    fn measure(&self, constraints: Constraints) -> Size {
        let content_width = self
            .items
            .iter()
            .map(|item| {
                2 + item.label.width()
                    + if item.detail.is_empty() {
                        0
                    } else {
                        2 + item.detail.width()
                    }
            })
            .chain(std::iter::once(self.title.width()))
            .chain(std::iter::once(self.footer.width()))
            .max()
            .unwrap_or(1);
        Size::new(
            (content_width as u16)
                .saturating_add(4)
                .min(constraints.max_width),
            (self.items.len() as u16)
                .saturating_add(3)
                .min(constraints.max_height),
        )
    }

    fn render(&self, area: Rect, buffer: &mut Buffer) {
        if area.width < 2 || area.height < 2 {
            return;
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(self.theme.border)
            .style(self.theme.surface)
            .padding(Padding::horizontal(1))
            .title(Span::styled(&self.title, self.theme.accent));
        let inner = block.inner(area);
        block.render(area, buffer);

        let mut lines = Vec::with_capacity(self.items.len() + 1);
        for (index, item) in self.items.iter().enumerate() {
            let style = if index == self.selected {
                self.theme.selection
            } else {
                self.theme.value
            };
            let marker = Span::styled("> ", self.theme.accent);
            let label = Span::styled(item.label.clone(), style);
            let detail = if item.detail.is_empty() {
                Span::raw("")
            } else {
                Span::styled(format!("  {}", item.detail), self.theme.muted)
            };
            lines.push(Line::from(vec![marker, label, detail]));
        }
        lines.push(Line::from(Span::styled(&self.footer, self.theme.muted)));

        RatatuiParagraph::new(lines)
            .style(self.theme.surface)
            .render(inner, buffer);
    }
}

fn line_width(line: &Line<'_>) -> usize {
    line.spans
        .iter()
        .map(|span| span.content.as_ref().width())
        .sum()
}

fn wrapped_line_height(line: &Line<'_>, width: u16) -> usize {
    if width == 0 {
        return 1;
    }
    line_width(line).max(1).div_ceil(width as usize)
}

fn block_dimensions(block: &Block<'_>, constraints: Constraints) -> (u16, u16) {
    let inner = block.inner(Rect::new(
        0,
        0,
        constraints.max_width,
        constraints.max_height,
    ));
    (
        constraints.max_width.saturating_sub(inner.width),
        constraints.max_height.saturating_sub(inner.height),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(scene: &Scene, width: u16) -> String {
        let size = scene.measure(Constraints::new(width, 20));
        let area = Rect::new(0, 0, size.width.max(1), size.height.max(1));
        let mut buffer = Buffer::empty(area);
        scene.render(area, &mut buffer);
        (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| buffer.cell((x, y)).unwrap().symbol().to_string())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn column_and_row_measure_natural_size() {
        let scene = Scene::new(
            Column::new().child(Text::new("Keel")).child(
                Row::new()
                    .gap(1)
                    .child(Text::new("a"))
                    .child(Text::new("bc")),
            ),
        );
        assert_eq!(scene.measure(Constraints::new(80, 20)), Size::new(4, 2));
    }

    #[test]
    fn popup_wraps_to_content_and_renders_selection() {
        let scene = Scene::new(SuggestionPopup::new(
            "suggestions",
            "1/2; Tab to accept",
            vec![
                PopupItem::new("--files-with-matches", ""),
                PopupItem::new("--files-without-match", ""),
            ],
            0,
            Theme::default(),
        ));
        let size = scene.measure(Constraints::new(80, 20));
        assert_eq!(size, Size::new(27, 5));
        let rendered = snapshot(&scene, 80);
        assert!(rendered.contains("suggestions"));
        assert!(rendered.contains("--files-with-matches"));
        assert!(rendered.contains("1/2; Tab to accept"));
    }

    #[test]
    fn popup_visual_snapshot_is_stable() {
        let scene = Scene::new(SuggestionPopup::new(
            "Keel suggestions",
            "1/2 · Up/Down to select",
            vec![
                PopupItem::new("--files-with-matches", "grep option"),
                PopupItem::new("--files-without-match", "grep option"),
            ],
            0,
            Theme::default(),
        ));

        insta::assert_snapshot!(snapshot(&scene, 42));
    }

    #[test]
    fn input_line_handles_unicode_cursor_positions() {
        let scene = Scene::new(InputLine::new("keel> ", "a🙂é", 2));
        assert_eq!(scene.measure(Constraints::new(80, 3)), Size::new(10, 1));
        let rendered = snapshot(&scene, 80);
        assert!(rendered.starts_with("keel> a🙂"));
    }
}
