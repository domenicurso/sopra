use keel_core::SuggestionKind;
use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalColors {
    pub background: Color,
    pub cursor: Color,
}

impl TerminalColors {
    pub const fn new(background: Color, cursor: Color) -> Self {
        Self { background, cursor }
    }

    pub fn cursor_style(self) -> Style {
        Style::default().fg(self.background).bg(self.cursor)
    }
}

impl Default for TerminalColors {
    fn default() -> Self {
        Self::new(Color::Reset, Color::White)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleToken {
    Match,
    Detail,
    Border,
    Footer,
    Selection,
    File,
    Directory,
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
            Self::Selection => Style::default().add_modifier(Modifier::REVERSED),
            Self::File => Style::default().fg(Color::LightYellow),
            Self::Directory => Style::default()
                .fg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
            Self::Cursor => TerminalColors::default().cursor_style(),
        }
    }
}

pub(crate) fn interpolate_style(
    from: Style,
    to: Style,
    amount: f32,
    colors: TerminalColors,
) -> Style {
    let amount = if amount.is_finite() {
        amount.clamp(0.0, 1.0)
    } else {
        1.0
    };
    if amount == 0.0 {
        return from;
    }
    if amount == 1.0 {
        return to;
    }

    let mut style = from;
    style.fg = interpolate_optional_color(from.fg, to.fg, amount, colors.background, colors.cursor);
    style.bg = interpolate_optional_color(from.bg, to.bg, amount, colors.background, colors.cursor);
    style.add_modifier = if amount < 0.5 {
        from.add_modifier
    } else {
        to.add_modifier
    };
    style.sub_modifier = if amount < 0.5 {
        from.sub_modifier
    } else {
        to.sub_modifier
    };
    style
}

fn interpolate_optional_color(
    from: Option<Color>,
    to: Option<Color>,
    amount: f32,
    from_default: Color,
    to_default: Color,
) -> Option<Color> {
    match (from, to) {
        (None, None) => None,
        (from, to) => {
            let from = color_rgb(from.unwrap_or(from_default));
            let to = color_rgb(to.unwrap_or(to_default));
            Some(Color::Rgb(
                interpolate_channel(from[0], to[0], amount),
                interpolate_channel(from[1], to[1], amount),
                interpolate_channel(from[2], to[2], amount),
            ))
        }
    }
}

fn interpolate_channel(from: u8, to: u8, amount: f32) -> u8 {
    (f32::from(from) + (f32::from(to) - f32::from(from)) * amount).round() as u8
}

fn color_rgb(color: Color) -> [u8; 3] {
    match color {
        Color::Reset => [0, 0, 0],
        Color::Black => [0, 0, 0],
        Color::Red => [128, 0, 0],
        Color::Green => [0, 128, 0],
        Color::Yellow => [128, 128, 0],
        Color::Blue => [0, 0, 128],
        Color::Magenta => [128, 0, 128],
        Color::Cyan => [0, 128, 128],
        Color::Gray => [192, 192, 192],
        Color::DarkGray => [128, 128, 128],
        Color::LightRed => [255, 0, 0],
        Color::LightGreen => [0, 255, 0],
        Color::LightYellow => [255, 255, 0],
        Color::LightBlue => [0, 0, 255],
        Color::LightMagenta => [255, 0, 255],
        Color::LightCyan => [0, 255, 255],
        Color::White => [255, 255, 255],
        Color::Indexed(index) => [index, index, index],
        Color::Rgb(red, green, blue) => [red, green, blue],
    }
}

pub(crate) fn role_style(kind: SuggestionKind) -> Style {
    match kind {
        SuggestionKind::Generic => Style::default(),
        SuggestionKind::File => StyleToken::File.style(),
        SuggestionKind::Directory => StyleToken::Directory.style(),
    }
}

pub(crate) fn matching_style(selected: bool) -> Style {
    let style = StyleToken::Match.style();
    if selected {
        style.add_modifier(Modifier::REVERSED)
    } else {
        style
    }
}

pub(crate) fn detail_style() -> Style {
    StyleToken::Detail.style()
}

pub(crate) fn footer_style() -> Style {
    StyleToken::Footer.style()
}

pub(crate) fn border_style() -> Style {
    StyleToken::Border.style()
}
