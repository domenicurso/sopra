use ratatui::{
    buffer::{Buffer, Cell},
    layout::Rect,
    style::{Color, Modifier, Style},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use keel_ui::{Scene, Size};

#[derive(Debug)]
pub struct RenderedFrame {
    pub area: Rect,
    pub used_size: Size,
    pub buffer: Buffer,
    pub content_area: Rect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameDiff {
    pub changed_cells: usize,
    pub changed_rows: Vec<u16>,
    pub previous_area: Rect,
    pub next_area: Rect,
    pub cleared_rows: u16,
}

impl FrameDiff {
    pub fn changed(&self) -> bool {
        self.changed_cells > 0 || self.cleared_rows > 0 || self.previous_area != self.next_area
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Renderer;

impl Renderer {
    pub fn render(&self, scene: &Scene, max_width: u16) -> RenderedFrame {
        self.render_in_width(scene, scene.measure(max_width.max(1)).width.max(1))
    }

    pub fn render_in_width(&self, scene: &Scene, width: u16) -> RenderedFrame {
        let width = width.max(1);
        let used_size = scene.measure(width);
        let area = Rect::new(0, 0, width, used_size.height.max(1));
        let mut buffer = Buffer::empty(area);
        scene.render(area, &mut buffer);
        let content_area = content_area(&buffer, area);
        RenderedFrame {
            area,
            used_size,
            buffer,
            content_area,
        }
    }

    pub fn diff(&self, previous: Option<&RenderedFrame>, next: &RenderedFrame) -> FrameDiff {
        let previous_area = previous.map_or(Rect::new(0, 0, 0, 0), |frame| frame.area);
        let width = previous_area.width.max(next.area.width);
        let height = previous_area.height.max(next.area.height);
        let mut changed_cells = 0;
        let mut changed_rows = Vec::new();
        let empty = Cell::EMPTY;

        for y in 0..height {
            let mut row_changed = false;
            for x in 0..width {
                let previous_cell = previous
                    .and_then(|frame| cell_at(&frame.buffer, frame.area, x, y))
                    .unwrap_or(&empty);
                let next_cell = cell_at(&next.buffer, next.area, x, y).unwrap_or(&empty);
                if previous_cell != next_cell {
                    changed_cells += 1;
                    row_changed = true;
                }
            }
            if row_changed {
                changed_rows.push(y);
            }
        }

        FrameDiff {
            changed_cells,
            changed_rows,
            previous_area,
            next_area: next.area,
            cleared_rows: previous_area.height.saturating_sub(next.area.height),
        }
    }

    pub fn to_zsh_prompt(&self, frame: &RenderedFrame) -> String {
        serialize_buffer(&frame.buffer, frame.content_area, true)
    }

    pub fn to_ansi(&self, frame: &RenderedFrame) -> String {
        serialize_buffer(&frame.buffer, frame.content_area, false)
    }
}

pub fn truncate_to_width(text: &str, max_width: u16) -> String {
    let max_width = max_width as usize;
    let mut used: usize = 0;
    let mut output = String::new();
    for grapheme in text.graphemes(true) {
        let width = UnicodeWidthStr::width(grapheme);
        if used.saturating_add(width) > max_width {
            break;
        }
        output.push_str(grapheme);
        used += width;
    }
    output
}

fn content_area(buffer: &Buffer, area: Rect) -> Rect {
    let mut min_x = area.width;
    let mut min_y = area.height;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut has_content = false;

    for y in 0..area.height {
        for x in 0..area.width {
            let cell = &buffer[(x, y)];
            if !has_visible_symbol(cell) {
                continue;
            }
            has_content = true;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x + 1);
            max_y = max_y.max(y + 1);
        }
    }

    if has_content {
        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    } else {
        Rect::new(0, 0, 0, 0)
    }
}

fn cell_at(buffer: &Buffer, area: Rect, x: u16, y: u16) -> Option<&Cell> {
    if x < area.width && y < area.height {
        Some(&buffer[(x, y)])
    } else {
        None
    }
}

fn serialize_buffer(buffer: &Buffer, area: Rect, zsh_wrapped: bool) -> String {
    if area.width == 0 || area.height == 0 {
        return String::new();
    }

    let mut output = String::new();
    for row in area.y..area.y + area.height {
        if row > area.y {
            output.push('\n');
        }
        let Some((first_column, last_column)) = row_content_bounds(buffer, area, row) else {
            continue;
        };
        let mut current_style = None;
        for column in first_column..=last_column {
            let cell = &buffer[(column, row)];
            let style = cell.style();
            if current_style != Some(style) {
                if current_style.is_some() {
                    push_control(&mut output, "\x1b[0m", zsh_wrapped);
                }
                push_control(&mut output, &style_to_ansi(style), zsh_wrapped);
                current_style = Some(style);
            }
            output.push_str(cell.symbol());
        }
        if current_style.is_some() {
            push_control(&mut output, "\x1b[0m", zsh_wrapped);
        }
    }
    output
}

fn row_content_bounds(buffer: &Buffer, area: Rect, row: u16) -> Option<(u16, u16)> {
    let mut first = None;
    let mut last = None;
    for column in area.x..area.x + area.width {
        if has_visible_symbol(&buffer[(column, row)]) {
            first.get_or_insert(column);
            last = Some(column);
        }
    }
    first.zip(last)
}

fn has_visible_symbol(cell: &Cell) -> bool {
    cell.symbol() != " "
}

fn push_control(output: &mut String, sequence: &str, zsh_wrapped: bool) {
    if zsh_wrapped {
        output.push_str("%{");
        output.push_str(sequence);
        output.push_str("%}");
    } else {
        output.push_str(sequence);
    }
}

fn style_to_ansi(style: Style) -> String {
    let mut codes = Vec::new();
    if style.add_modifier.contains(Modifier::BOLD) {
        codes.push("1".to_string());
    }
    if let Some(code) = color_to_ansi(style.fg, false) {
        codes.push(code);
    }
    if let Some(code) = color_to_ansi(style.bg, true) {
        codes.push(code);
    }
    if codes.is_empty() {
        "\x1b[0m".to_string()
    } else {
        format!("\x1b[{}m", codes.join(";"))
    }
}

fn color_to_ansi(color: Option<Color>, background: bool) -> Option<String> {
    let prefix = if background { 48 } else { 38 };
    match color? {
        Color::Reset => None,
        Color::Rgb(red, green, blue) => Some(format!("{prefix};2;{red};{green};{blue}")),
        Color::Indexed(index) => Some(format!("{prefix};5;{index}")),
        Color::Black => Some(format!("{prefix};5;0")),
        Color::Red => Some(format!("{prefix};5;1")),
        Color::Green => Some(format!("{prefix};5;2")),
        Color::Yellow => Some(format!("{prefix};5;3")),
        Color::Blue => Some(format!("{prefix};5;4")),
        Color::Magenta => Some(format!("{prefix};5;5")),
        Color::Cyan => Some(format!("{prefix};5;6")),
        Color::Gray => Some(format!("{prefix};5;7")),
        Color::DarkGray => Some(format!("{prefix};5;8")),
        Color::LightRed => Some(format!("{prefix};5;9")),
        Color::LightGreen => Some(format!("{prefix};5;10")),
        Color::LightYellow => Some(format!("{prefix};5;11")),
        Color::LightBlue => Some(format!("{prefix};5;12")),
        Color::LightMagenta => Some(format!("{prefix};5;13")),
        Color::LightCyan => Some(format!("{prefix};5;14")),
        Color::White => Some(format!("{prefix};5;15")),
    }
}

#[cfg(test)]
mod tests {
    use keel_ui::{Paragraph, Scene, Text, Theme, badge_scene};
    use ratatui::{
        buffer::Buffer,
        layout::Rect,
        style::{Color, Style},
    };

    use super::{FrameDiff, Renderer};

    #[test]
    fn renders_ratatui_styling_as_a_zsh_fragment() {
        let renderer = Renderer;
        let frame = renderer.render(&badge_scene("Keel | main | 7c:5"), 80);
        let fragment = renderer.to_zsh_prompt(&frame);

        assert!(fragment.contains("Keel | main | 7c:5"));
        assert!(fragment.contains("%{\u{1b}["));
        assert!(fragment.contains("38;2;125;211;252"));
        assert!(!fragment.contains('\n'));
    }

    #[test]
    fn keeps_the_widget_inside_the_terminal_width() {
        let renderer = Renderer;
        let frame = renderer.render(&badge_scene("a compact badge"), 8);

        assert!(frame.used_size.width <= 8);
        assert_eq!(frame.used_size.height, 1);
    }

    #[test]
    fn diff_reports_changed_cells_and_row_cleanup() {
        let renderer = Renderer;
        let first = renderer.render(
            &Scene::new(Paragraph::new("one\ntwo").style(Style::default().fg(Color::Red))),
            10,
        );
        let second = renderer.render(&Scene::new(Text::new("one")), 10);

        let diff = renderer.diff(Some(&first), &second);
        assert!(diff.changed());
        assert!(diff.changed_cells > 0);
        assert_eq!(diff.cleared_rows, 1);
    }

    #[test]
    fn empty_buffer_has_no_prompt_fragment() {
        let renderer = Renderer;
        let frame = renderer.render(&Scene::new(Text::new("")), 10);
        assert_eq!(renderer.to_zsh_prompt(&frame), "");
    }

    #[test]
    fn keeps_wide_graphemes_whole_when_truncating() {
        assert_eq!(super::truncate_to_width("ab界c", 3), "ab");
        assert_eq!(super::truncate_to_width("ab界c", 4), "ab界");
    }

    #[allow(dead_code)]
    fn _buffer_type_is_ratatuified() {
        let _ = Buffer::empty(Rect::new(0, 0, 1, 1));
        let _ = Theme::default();
        let _ = FrameDiff {
            changed_cells: 0,
            changed_rows: vec![],
            previous_area: Rect::default(),
            next_area: Rect::default(),
            cleared_rows: 0,
        };
    }
}
