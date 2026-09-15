use ratatui::style::{Color, Modifier, Style};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::{
    Element,
    canvas::{Canvas, TextRun},
};

pub(super) struct PromptElement {
    pub(super) column: u16,
    pub(super) row: u16,
    pub(super) text: String,
    pub(super) style: Style,
}

impl Element for PromptElement {
    fn paint(&self, canvas: &mut Canvas) {
        canvas.text(TextRun {
            position: (self.column, self.row),
            text: &self.text,
            style: self.style,
            max_width: canvas.buffer.area.width.saturating_sub(self.column),
        });
    }
}

pub(super) struct CommandElement {
    pub(super) column: u16,
    pub(super) row: u16,
    pub(super) buffer: String,
}

impl Element for CommandElement {
    fn paint(&self, canvas: &mut Canvas) {
        let width = canvas.buffer.area.width.saturating_sub(self.column);
        let mut offset = 0_u16;
        let mut first_word = true;
        let mut token_is_flag = false;
        for grapheme in self.buffer.graphemes(true) {
            if grapheme.chars().any(char::is_control) {
                continue;
            }
            let grapheme_width = grapheme.width() as u16;
            if grapheme_width == 0 {
                continue;
            }
            if grapheme.chars().all(char::is_whitespace) {
                first_word = false;
                token_is_flag = false;
            } else if first_word {
                token_is_flag = grapheme == "-";
            }
            let style = command_style(first_word, token_is_flag || grapheme == "-");
            canvas.text(TextRun {
                position: (self.column.saturating_add(offset), self.row),
                text: grapheme,
                style,
                max_width: width.saturating_sub(offset),
            });
            offset = offset.saturating_add(grapheme_width);
        }
    }
}

fn command_style(first_word: bool, flag: bool) -> Style {
    if first_word {
        Style::default()
            .fg(Color::Rgb(125, 196, 255))
            .add_modifier(Modifier::BOLD)
    } else if flag {
        Style::default().fg(Color::Rgb(248, 195, 111))
    } else {
        Style::default().fg(Color::Rgb(220, 224, 232))
    }
}

pub(super) struct CursorElement {
    pub(super) column: u16,
    pub(super) row: u16,
    pub(super) width: u16,
    pub(super) style: Style,
}

impl Element for CursorElement {
    fn paint(&self, canvas: &mut Canvas) {
        for offset in 0..self.width.max(1) {
            canvas.put(
                self.column.saturating_add(offset),
                self.row,
                " ",
                self.style,
            );
        }
    }
}

pub(super) fn display_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}
