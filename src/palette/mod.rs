mod parser;
mod query;

use std::os::fd::RawFd;

use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Rgb {
    red: u8,
    green: u8,
    blue: u8,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct TerminalPalette {
    background: Option<Rgb>,
    cursor: Option<Rgb>,
}

impl TerminalPalette {
    pub(crate) fn query(fd: RawFd) -> (Self, Vec<u8>) {
        query::read(fd)
    }

    pub(crate) fn cursor_style(self, opacity: f32) -> Style {
        let Some(background) = self.background else {
            return fallback_cursor_style(opacity);
        };
        let Some(cursor) = self.cursor else {
            return fallback_cursor_style(opacity);
        };
        let color = blend(background, cursor, opacity);
        Style::default()
            .fg(Color::Rgb(
                background.red,
                background.green,
                background.blue,
            ))
            .bg(Color::Rgb(color.red, color.green, color.blue))
    }
}

fn blend(start: Rgb, end: Rgb, opacity: f32) -> Rgb {
    let opacity = opacity.clamp(0.0, 1.0);
    Rgb {
        red: interpolate(start.red, end.red, opacity),
        green: interpolate(start.green, end.green, opacity),
        blue: interpolate(start.blue, end.blue, opacity),
    }
}

fn interpolate(start: u8, end: u8, amount: f32) -> u8 {
    (f32::from(start) + (f32::from(end) - f32::from(start)) * amount).round() as u8
}

fn fallback_cursor_style(opacity: f32) -> Style {
    let mut style = Style::default().add_modifier(Modifier::REVERSED);
    if opacity < 0.7 {
        style = style.add_modifier(Modifier::DIM);
    }
    style
}
