use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph as RatatuiParagraph, Widget, Wrap},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub const MAX_VISIBLE_ITEMS: usize = 12;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleToken {
    Match,
    Detail,
    Border,
    Footer,
    Selection,
    Cursor,
}

impl StyleToken {
    pub fn style(self) -> Style {
        match self {
            Self::Match => Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
            Self::Detail | Self::Border | Self::Footer => Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::DIM),
            Self::Selection | Self::Cursor => Style::default().add_modifier(Modifier::REVERSED),
        }
    }
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
            cursor_style: StyleToken::Cursor.style(),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PopupPlacement {
    #[default]
    Below,
    Above,
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
    footer: String,
    items: Vec<PopupItem>,
    selected: Option<usize>,
    query: String,
    anchor_column: u16,
    placement: PopupPlacement,
    viewport_start: usize,
    max_visible_items: usize,
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
        let scrollbar_width = usize::from(self.items.len() > self.max_visible_items);
        let visible_items = self.items.len().min(self.max_visible_items);
        Size::new(
            (content_width as u16)
                .saturating_add(4)
                .saturating_add(scrollbar_width as u16)
                .min(constraints.max_width),
            (visible_items as u16)
                .saturating_add(2)
                .min(constraints.max_height),
        )
    }

    fn render(&self, area: Rect, buffer: &mut Buffer) {
        if area.width < 2 || area.height < 2 {
            return;
        }

        let connector_column = self.anchor_column.clamp(1, area.width.saturating_sub(2));
        let junction = match self.placement {
            PopupPlacement::Below => "┴",
            PopupPlacement::Above => "┬",
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style())
            .padding(Padding::horizontal(1))
            .title_bottom(
                Line::from(Span::styled(self.footer.clone(), footer_style())).right_aligned(),
            );
        let inner = block.inner(area);
        block.render(area, buffer);

        let junction_column = area.x + connector_column;
        let junction_row = match self.placement {
            PopupPlacement::Below => area.y,
            PopupPlacement::Above => area.y + area.height - 1,
        };
        if let Some(cell) = buffer.cell_mut((junction_column, junction_row)) {
            cell.set_symbol(junction);
            cell.set_style(border_style());
        }

        let mut lines = Vec::with_capacity(self.items.len());
        let visible_count = self
            .items
            .len()
            .min(self.max_visible_items)
            .min(inner.height as usize);
        let mut viewport_start = self
            .viewport_start
            .min(self.items.len().saturating_sub(visible_count));
        if let Some(selected) = self.selected {
            if selected < viewport_start {
                viewport_start = selected;
            } else if selected >= viewport_start.saturating_add(visible_count) && visible_count > 0
            {
                viewport_start = selected.saturating_add(1).saturating_sub(visible_count);
            }
        }
        let end = viewport_start
            .saturating_add(visible_count)
            .min(self.items.len());
        for (index, item) in self.items[viewport_start..end].iter().enumerate() {
            let index = viewport_start + index;
            let selected = self.selected == Some(index);
            let term_style = if selected {
                StyleToken::Selection.style()
            } else {
                Style::default()
            };
            let mut spans = match_spans(
                &item.label,
                &self.query,
                term_style,
                matching_style(selected),
            );
            let detail = if item.detail.is_empty() {
                Span::raw("")
            } else {
                Span::styled(format!("  {}", item.detail), detail_style())
            };
            spans.push(detail);
            lines.push(Line::from(spans));
        }

        let has_scrollbar = self.items.len() > self.max_visible_items;
        let content_area = if has_scrollbar && inner.width > 0 {
            Rect::new(
                inner.x,
                inner.y,
                inner.width.saturating_sub(1),
                inner.height,
            )
        } else {
            inner
        };
        RatatuiParagraph::new(lines).render(content_area, buffer);
        if has_scrollbar {
            render_scrollbar(
                Rect::new(
                    inner.x.saturating_add(inner.width.saturating_sub(1)),
                    inner.y,
                    1,
                    inner.height,
                ),
                self.items.len(),
                viewport_start,
                visible_count,
                buffer,
            );
        }
    }
}

fn render_scrollbar(
    area: Rect,
    item_count: usize,
    viewport_start: usize,
    visible_count: usize,
    buffer: &mut Buffer,
) {
    if area.width == 0 || area.height == 0 || item_count == 0 || visible_count == 0 {
        return;
    }

    let track_height = area.height as usize;
    let thumb_height = (track_height * visible_count).div_ceil(item_count).max(1);
    let max_thumb_top = track_height.saturating_sub(thumb_height);
    let max_viewport_start = item_count.saturating_sub(visible_count);
    let thumb_top = if max_viewport_start == 0 {
        0
    } else {
        viewport_start.min(max_viewport_start) * max_thumb_top / max_viewport_start
    };
    for row in 0..track_height {
        let Some(cell) = buffer.cell_mut((area.x, area.y + row as u16)) else {
            continue;
        };
        cell.set_symbol(" ");
        if (thumb_top..thumb_top + thumb_height).contains(&row) {
            cell.set_style(
                Style::default()
                    .bg(Color::White)
                    .add_modifier(Modifier::DIM),
            );
        } else {
            cell.set_style(Style::default());
        }
    }
}

fn matching_style(selected: bool) -> Style {
    let style = StyleToken::Match.style();
    if selected {
        style.add_modifier(Modifier::REVERSED)
    } else {
        style
    }
}

fn detail_style() -> Style {
    StyleToken::Detail.style()
}

fn footer_style() -> Style {
    StyleToken::Footer.style()
}

fn border_style() -> Style {
    StyleToken::Border.style()
}

fn match_spans(text: &str, query: &str, base: Style, matching: Style) -> Vec<Span<'static>> {
    let graphemes = text.graphemes(true).collect::<Vec<_>>();
    let mut matched = vec![false; graphemes.len()];
    let mut search_start = 0;
    for wanted in query
        .graphemes(true)
        .filter(|grapheme| !grapheme.chars().all(char::is_whitespace))
    {
        let Some(index) = graphemes[search_start..]
            .iter()
            .position(|grapheme| grapheme.eq_ignore_ascii_case(wanted))
            .map(|index| index + search_start)
        else {
            continue;
        };
        matched[index] = true;
        search_start = index + 1;
    }

    let mut spans = Vec::new();
    let mut current = String::new();
    let mut current_style = base;
    for (index, grapheme) in graphemes.iter().enumerate() {
        let style = if matched[index] { matching } else { base };
        if style != current_style && !current.is_empty() {
            spans.push(Span::styled(std::mem::take(&mut current), current_style));
        }
        current_style = style;
        current.push_str(grapheme);
    }
    if !current.is_empty() {
        spans.push(Span::styled(current, current_style));
    }
    spans
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
    fn composition_measures_context_multiline_body_and_input() {
        let scene = Scene::new(
            Column::new()
                .child(Text::new("context"))
                .child(Paragraph::new("first\nsecond").wrap(false))
                .child(InputLine::new("> ", "open", 4)),
        );

        assert_eq!(scene.measure(Constraints::new(80, 20)), Size::new(7, 4));
    }

    #[test]
    fn popup_wraps_to_content_and_renders_selection() {
        let scene = Scene::new(SuggestionPopup::new(
            "1/2; Tab to accept",
            vec![
                PopupItem::new("--files-with-matches", ""),
                PopupItem::new("--files-without-match", ""),
            ],
            Some(0),
        ));
        let size = scene.measure(Constraints::new(80, 20));
        assert_eq!(size, Size::new(25, 4));
        let rendered = snapshot(&scene, 80);
        assert!(!rendered.contains("suggestions"));
        assert!(!rendered.contains("> "));
        assert!(rendered.contains("--files-with-matches"));
        assert!(rendered.contains("1/2; Tab to accept"));
    }

    #[test]
    fn selection_reverses_only_the_term_and_keeps_details_dim() {
        let scene = Scene::new(
            SuggestionPopup::new(
                "1/1; 2ms",
                vec![PopupItem::new("alpha", "description")],
                Some(0),
            )
            .query("a"),
        );
        let size = scene.measure(Constraints::new(80, 20));
        let area = Rect::new(0, 0, size.width, size.height);
        let mut buffer = Buffer::empty(area);
        scene.render(area, &mut buffer);

        let term_style = buffer.cell((2, 1)).unwrap().style();
        let detail_style = buffer.cell((9, 1)).unwrap().style();
        assert!(term_style.add_modifier.contains(Modifier::REVERSED));
        assert!(term_style.add_modifier.contains(Modifier::UNDERLINED));
        assert!(detail_style.add_modifier.contains(Modifier::DIM));
        assert!(!detail_style.add_modifier.contains(Modifier::REVERSED));
    }

    #[test]
    fn popup_visual_snapshot_is_stable() {
        let scene = Scene::new(SuggestionPopup::new(
            "1/2; Up/Down to select",
            vec![
                PopupItem::new("--files-with-matches", "grep option"),
                PopupItem::new("--files-without-match", "grep option"),
            ],
            Some(0),
        ));

        insta::assert_snapshot!(snapshot(&scene, 42));
    }

    #[test]
    fn popup_limits_entries_and_renders_an_inline_scrollbar() {
        let items = (0..20)
            .map(|index| PopupItem::new(format!("item-{index:02}"), "detail"))
            .collect();
        let scene = Scene::new(
            SuggestionPopup::new("11/20; 3ms", items, Some(10)).viewport(1, MAX_VISIBLE_ITEMS),
        );
        let size = scene.measure(Constraints::new(80, 40));
        assert_eq!(size.height, 14);
        let area = Rect::new(0, 0, size.width, size.height);
        let mut buffer = Buffer::empty(area);
        scene.render(area, &mut buffer);

        let rendered = snapshot(&scene, 80);
        assert!(rendered.contains("item-01"));
        assert!(rendered.contains("item-12"));
        assert!(!rendered.contains("item-00"));
        assert!(!rendered.contains("item-13"));

        let scrollbar_x = area.x + area.width - 3;
        let thumb = buffer.cell((scrollbar_x, area.y + 5)).unwrap();
        assert_eq!(thumb.symbol(), " ");
        assert_eq!(thumb.bg, Color::White);
    }

    #[test]
    fn input_line_handles_unicode_cursor_positions() {
        let scene = Scene::new(InputLine::new("keel> ", "a🙂é", 2));
        assert_eq!(scene.measure(Constraints::new(80, 3)), Size::new(10, 1));
        let rendered = snapshot(&scene, 80);
        assert!(rendered.starts_with("keel> a🙂"));
    }
}
