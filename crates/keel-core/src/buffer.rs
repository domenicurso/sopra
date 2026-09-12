use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorBuffer {
    text: String,
    cursor: usize,
}

impl EditorBuffer {
    pub fn new(text: impl Into<String>, cursor: usize) -> Self {
        let text = text.into();
        let cursor = cursor.min(text.graphemes(true).count());
        Self { text, cursor }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn grapheme_count(&self) -> usize {
        self.text.graphemes(true).count()
    }

    pub fn cursor_byte_offset(&self) -> usize {
        self.text
            .grapheme_indices(true)
            .nth(self.cursor)
            .map_or(self.text.len(), |(offset, _)| offset)
    }

    pub fn cursor_width(&self) -> usize {
        self.text
            .graphemes(true)
            .take(self.cursor)
            .collect::<String>()
            .width()
    }

    pub fn insert(&mut self, value: &str) -> bool {
        if value.is_empty() {
            return false;
        }

        let offset = self.cursor_byte_offset();
        self.text.insert_str(offset, value);
        self.cursor += value.graphemes(true).count();
        true
    }

    pub fn backspace(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }

        let graphemes = self.text.graphemes(true).collect::<Vec<_>>();
        self.text = graphemes[..self.cursor - 1]
            .iter()
            .chain(graphemes[self.cursor..].iter())
            .copied()
            .collect();
        self.cursor -= 1;
        true
    }

    pub fn delete(&mut self) -> bool {
        if self.cursor >= self.grapheme_count() {
            return false;
        }

        let graphemes = self.text.graphemes(true).collect::<Vec<_>>();
        self.text = graphemes[..self.cursor]
            .iter()
            .chain(graphemes[self.cursor + 1..].iter())
            .copied()
            .collect();
        true
    }

    pub fn move_left(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        true
    }

    pub fn move_right(&mut self) -> bool {
        if self.cursor >= self.grapheme_count() {
            return false;
        }
        self.cursor += 1;
        true
    }

    pub fn move_home(&mut self) -> bool {
        let changed = self.cursor != 0;
        self.cursor = 0;
        changed
    }

    pub fn move_end(&mut self) -> bool {
        let end = self.grapheme_count();
        let changed = self.cursor != end;
        self.cursor = end;
        changed
    }
}
