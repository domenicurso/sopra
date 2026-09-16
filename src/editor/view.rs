use std::time::{Duration, Instant};

use ratatui::style::{Modifier, Style};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{
    completion::CompletionItem,
    input::{CursorPosition, TerminalSize},
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

    pub(crate) fn completion_token_width(&self) -> u16 {
        UnicodeWidthStr::width(self.query()).min(u16::MAX as usize) as u16
    }

    pub(crate) fn visible_items(&self) -> &[CompletionItem] {
        &self.suggestions
    }

    pub(crate) fn syntax(&self) -> &[crate::syntax::SyntaxSpan] {
        &self.syntax
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
        self.palette
            .cursor_style(crate::animation::cursor_opacity(
                self.animation_elapsed(now),
            ))
            .add_modifier(Modifier::BOLD)
    }

    fn position_for(&self, size: TerminalSize, prompt: &str, content_width: u16) -> CursorPosition {
        let row = self.anchor.row.min(size.rows.saturating_sub(1));
        let prompt_width = crate::prompt::width(&crate::prompt::parse(prompt)) as u16;
        let column = self
            .anchor
            .column
            .saturating_add(prompt_width)
            .saturating_add(content_width)
            .min(size.columns.saturating_sub(1));
        CursorPosition { row, column }
    }
}
