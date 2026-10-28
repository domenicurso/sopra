use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::{
    Element,
    canvas::{Canvas, TextRun},
    editor_layout::EditorLayout,
};
use crate::{editor::EditorState, prompt::PromptSpan};

pub(super) struct Autosuggestion {
    column: u16,
    row: u16,
    end_column: u16,
    spans: Vec<PromptSpan>,
}

impl Autosuggestion {
    pub(super) fn new(editor: &EditorState, layout: &EditorLayout) -> Self {
        Self {
            column: layout.cursor_column,
            row: layout.cursor_row,
            end_column: if layout.cursor_row == layout.prompt_row {
                layout.rprompt_column
            } else {
                layout.area.width
            },
            spans: editor.autosuggestion_spans(),
        }
    }
}

impl Element for Autosuggestion {
    fn paint(&self, canvas: &mut Canvas) {
        let mut column = self.column;
        for span in &self.spans {
            for grapheme in span.text.graphemes(true) {
                let width = grapheme.width() as u16;
                if grapheme.chars().any(char::is_control)
                    || column.saturating_add(width) > self.end_column
                {
                    return;
                }
                canvas.text(TextRun {
                    position: (column, self.row),
                    text: grapheme,
                    style: span.style,
                    max_width: self.end_column.saturating_sub(column),
                });
                column = column.saturating_add(width);
            }
        }
    }
}
