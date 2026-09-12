use ratatui::style::{Color, Modifier, Style};

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
