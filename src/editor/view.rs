use std::time::{Duration, Instant};

use ratatui::style::{Modifier, Style};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::input::{CursorPosition, TerminalSize};
use sopra_completion::CompletionItem;

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

    pub(crate) fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    pub(crate) const fn suggestion_viewport_start(&self) -> usize {
        self.suggestion_scroll
    }

    pub(crate) fn overlay_visible(&self) -> bool {
        self.overlay_visible
    }

    pub(crate) fn completion_query_for(&self, item: &CompletionItem) -> String {
        sopra_completion::ranking::replacement_query(&self.buffer, self.cursor, item).to_string()
    }

    pub(crate) fn completion_token_width(&self) -> u16 {
        let query = self
            .suggestions
            .first()
            .map_or_else(String::new, |item| self.completion_query_for(item));
        sopra_completion::ranking::completion_token_width(&self.suggestions, &query)
            .min(u16::MAX as usize) as u16
    }

    pub(crate) fn visible_items(&self) -> &[CompletionItem] {
        &self.suggestions
    }

    pub(crate) fn completion_footer(&self) -> String {
        format!(
            "{}/{}; {}",
            self.selected.map_or(0, |index| index + 1),
            self.suggestions.len(),
            completion_elapsed(self.completion_elapsed)
        )
    }

    pub(crate) fn completion_hint(&self) -> &'static str {
        if self.selected.is_some() || self.suggestions.len() == 1 {
            "Tab to accept"
        } else {
            "Tab to focus"
        }
    }

    pub(crate) fn syntax(&self) -> &[crate::syntax::SyntaxSpan] {
        &self.syntax
    }

    pub(crate) fn transient_syntax(&self) -> Vec<crate::syntax::SyntaxSpan> {
        crate::syntax::highlight_without_cursor(&self.buffer, &self.cwd)
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

    pub(crate) fn cursor_symbol(&self) -> String {
        self.buffer[self.cursor..]
            .graphemes(true)
            .next()
            .filter(|grapheme| {
                !grapheme.chars().any(char::is_control) && UnicodeWidthStr::width(*grapheme) > 0
            })
            .map_or_else(|| " ".to_string(), ToString::to_string)
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

fn completion_elapsed(elapsed: Duration) -> String {
    if elapsed.is_zero() {
        return "pending".to_string();
    }
    let millis = elapsed.as_secs_f64() * 1_000.0;
    if millis < 0.1 {
        "<0.1ms".to_string()
    } else {
        format!("{millis:.1}ms")
    }
}
