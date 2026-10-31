use super::EditorState;

impl EditorState {
    pub(super) fn insert_paste(&mut self, text: &str) {
        if let Some(range) = self.text_selection.range() {
            self.cursor = range.start;
            self.buffer.replace_range(range, "");
        }
        self.buffer.insert_str(self.cursor, text);
        self.cursor += text.len();
        self.text_selection.clear();
        self.resume_completion();
        self.note_activity();
        self.request_completion();
    }
}
