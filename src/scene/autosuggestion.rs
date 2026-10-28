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
    metadata: Vec<PromptSpan>,
}

impl Autosuggestion {
    pub(super) fn new(editor: &EditorState, layout: &EditorLayout) -> Self {
        let spans = editor.autosuggestion_spans();
        let metadata = if spans.is_empty() {
            Vec::new()
        } else {
            editor.autosuggestion_metadata()
        };
        Self {
            column: layout.cursor_column,
            row: layout.cursor_row,
            end_column: if layout.cursor_row == layout.prompt_row {
                layout.rprompt_column
            } else {
                layout.area.width
            },
            spans,
            metadata,
        }
    }
}

impl Element for Autosuggestion {
    fn paint(&self, canvas: &mut Canvas) {
        if self.spans.is_empty() {
            return;
        }
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
        let width = crate::prompt::width(&self.metadata).min(u16::MAX as usize) as u16;
        if !self.spans.is_empty()
            && width > 0
            && column.saturating_add(2).saturating_add(width) <= self.end_column
        {
            super::elements::PromptElement {
                column: column + 2,
                row: self.row,
                spans: self.metadata.clone(),
            }
            .paint(canvas);
        }
    }
}
