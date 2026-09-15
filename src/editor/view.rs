use std::time::{Duration, Instant};

use ratatui::style::{Color, Modifier, Style};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{
    input::{CursorPosition, TerminalSize},
    scene::{DemoItem, demo_items},
};

use super::EditorState;

impl EditorState {
    pub(crate) fn buffer(&self) -> &str {
        &self.buffer
    }

    pub(crate) fn prompt(&self) -> &str {
        &self.prompt
    }

    pub(crate) const fn anchor(&self) -> CursorPosition {
        self.anchor
    }

    pub(crate) fn selected_index(&self) -> usize {
        self.selected
    }

    pub(crate) fn overlay_visible(&self) -> bool {
        self.overlay_visible
    }

    pub(crate) fn query(&self) -> &str {
        let (start, end) = self.current_token_range();
        &self.buffer[start..end]
    }

    pub(crate) fn visible_items(&self) -> Vec<DemoItem> {
        demo_items(self.query())
    }

    pub(crate) fn cursor_display_width(&self) -> u16 {
        UnicodeWidthStr::width(&self.buffer[..self.cursor]) as u16
    }

    pub(crate) fn cursor_cell_width(&self) -> u16 {
        self.buffer[self.cursor..]
            .graphemes(true)
            .next()
            .map(|grapheme| UnicodeWidthStr::width(grapheme).max(1) as u16)
            .unwrap_or(1)
    }

    pub(crate) fn status_text(&self) -> String {
        format!("{} · 60fps", self.status)
    }

    pub(crate) fn animation_elapsed(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.animation_started)
    }

    pub(super) fn restore_position(&self, size: TerminalSize) -> CursorPosition {
        self.position_for(size, self.prompt(), self.cursor_display_width())
    }

    pub(super) fn restore_prompt_position(
        &self,
        size: TerminalSize,
        prompt: &str,
    ) -> CursorPosition {
        self.position_for(size, prompt, 0)
    }

    pub(crate) fn cursor_style(&self, now: Instant) -> Style {
        let pulse =
            ((self.animation_elapsed(now).as_secs_f32() * std::f32::consts::TAU * 1.2).sin() + 1.0)
                / 2.0;
        Style::default()
            .fg(Color::Rgb(8, 16, 22))
            .bg(Color::Rgb(
                interpolate(36, 107, pulse),
                interpolate(139, 221, pulse),
                interpolate(165, 205, pulse),
            ))
            .add_modifier(Modifier::BOLD)
    }

    fn position_for(&self, size: TerminalSize, prompt: &str, content_width: u16) -> CursorPosition {
        let row = self.anchor.row.min(size.rows.saturating_sub(1));
        let column = self
            .anchor
            .column
            .saturating_add(UnicodeWidthStr::width(prompt) as u16)
            .saturating_add(content_width)
            .min(size.columns.saturating_sub(1));
        CursorPosition { row, column }
    }
}

fn interpolate(start: u8, end: u8, amount: f32) -> u8 {
    (f32::from(start) + f32::from(end - start) * amount).round() as u8
}
