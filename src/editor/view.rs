use std::time::{Duration, Instant};

use ratatui::style::{Modifier, Style};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::input::{CursorPosition, TerminalSize};

use super::EditorState;

impl EditorState {
    pub(crate) fn text_selection(&self) -> Option<std::ops::Range<usize>> {
        self.text_selection.range()
    }

    pub(crate) fn cursor_byte(&self) -> usize {
        self.cursor
    }

    pub(crate) fn buffer(&self) -> &str {
        &self.buffer
    }

    pub(crate) fn prompt(&self) -> &[crate::prompt::PromptSpan] {
        &self.prompt
    }

    pub(crate) fn set_transient_prompt(&mut self, prompt: String) {
        self.transient_prompt = prompt;
    }

    pub(crate) fn set_secondary_prompt(&mut self, prompt: String) {
        self.secondary_prompt = prompt;
    }

    pub(crate) fn secondary_prompt(&self) -> &str {
        &self.secondary_prompt
    }

    pub(crate) fn transient_prompt(&self) -> &str {
        &self.transient_prompt
    }

    pub(crate) fn rprompt(&self) -> &[crate::prompt::PromptSpan] {
        &self.rprompt
    }

    pub(crate) const fn anchor(&self) -> CursorPosition {
        self.anchor
    }

    pub(crate) fn syntax(&self) -> &[crate::syntax::SyntaxSpan] {
        &self.syntax
    }

    pub(crate) fn transient_syntax(&self) -> Vec<crate::syntax::SyntaxSpan> {
        if !self.syntax_enabled {
            return Vec::new();
        }
        crate::syntax::highlight_without_cursor(&self.buffer, &self.cwd)
    }

    pub(crate) fn cursor_cell_width(&self) -> u16 {
        self.cursor_text()
            .graphemes(true)
            .next()
            .map(|grapheme| UnicodeWidthStr::width(grapheme).max(1) as u16)
            .unwrap_or(1)
    }

    pub(crate) fn cursor_symbol(&self) -> String {
        self.cursor_text()
            .graphemes(true)
            .next()
            .filter(|grapheme| {
                !grapheme.chars().any(char::is_control) && UnicodeWidthStr::width(*grapheme) > 0
            })
            .map_or_else(|| " ".to_string(), ToString::to_string)
    }

    fn cursor_text(&self) -> &str {
        if self.cursor == self.buffer.len() {
            self.history_suffix().unwrap_or("")
        } else {
            &self.buffer[self.cursor..]
        }
    }

    pub(crate) fn animation_elapsed(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.animation_started)
    }

    pub(crate) fn cursor_animation_enabled(&self) -> bool {
        self.cursor_animation
    }

    pub(super) fn restore_position(&self, size: TerminalSize) -> CursorPosition {
        self.text_layout(size).position(self.cursor)
    }

    pub(super) fn restore_prompt_position(
        &self,
        size: TerminalSize,
        prompt: &[crate::prompt::PromptSpan],
    ) -> CursorPosition {
        self.position_for(size, prompt, false)
    }

    pub(crate) fn cursor_style(&self, now: Instant) -> Style {
        self.palette
            .cursor_style(if self.cursor_animation {
                crate::animation::cursor_opacity(self.animation_elapsed(now))
            } else {
                1.0
            })
            .add_modifier(Modifier::BOLD)
    }

    fn position_for(
        &self,
        size: TerminalSize,
        prompt: &[crate::prompt::PromptSpan],
        include_buffer: bool,
    ) -> CursorPosition {
        let mut flow = crate::text::flow::TextFlow::new(self.anchor, size.columns);
        flow.prompt(prompt);
        if include_buffer {
            flow.command(
                &self.buffer[..self.cursor],
                &crate::prompt::parse(self.secondary_prompt()),
            );
        }
        flow.cursor()
    }
}
