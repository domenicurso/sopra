use keel_ui::{Scene, Size};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
};

#[derive(Debug)]
pub struct RenderedFrame {
    pub area: Rect,
    pub used_size: Size,
    pub buffer: Buffer,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Renderer;

impl Renderer {
    pub fn render(&self, scene: &Scene, max_width: u16) -> RenderedFrame {
        let used_size = scene.measure(max_width.max(1));
        let area = Rect::new(0, 0, used_size.width.max(1), used_size.height.max(1));
        let mut buffer = Buffer::empty(area);
        scene.render(area, &mut buffer);
        RenderedFrame {
            area,
            used_size,
            buffer,
        }
    }

    pub fn to_zsh_prompt(&self, frame: &RenderedFrame) -> String {
        serialize_buffer(&frame.buffer, frame.area)
    }
}

fn serialize_buffer(buffer: &Buffer, area: Rect) -> String {
    let mut output = String::new();
    let mut current_style = None;

    for column in 0..area.width {
        let cell = &buffer[(column, 0)];
        let style = cell.style();
        if current_style != Some(style) {
            if current_style.is_some() {
                push_control(&mut output, "\x1b[0m");
            }
            push_control(&mut output, &style_to_ansi(style));
            current_style = Some(style);
        }
        output.push_str(cell.symbol());
    }

    if current_style.is_some() {
        push_control(&mut output, "\x1b[0m");
    }
    output
}

fn push_control(output: &mut String, sequence: &str) {
    output.push_str("%{");
    output.push_str(sequence);
    output.push_str("%}");
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
    use keel_ui::badge_scene;

    use super::Renderer;

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
}
