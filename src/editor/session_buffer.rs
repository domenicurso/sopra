use super::EditorState;

pub(crate) struct Draft {
    pub(crate) buffer: String,
    pub(crate) cursor: usize,
}
impl EditorState {
    pub(super) fn replace_from_session(&mut self, text: String, execute: bool) {
        if execute && self.draft.is_none() && !self.buffer.is_empty() {
            self.draft = Some(Draft {
                buffer: std::mem::take(&mut self.buffer),
                cursor: self.cursor,
            });
        }
        self.buffer = text;
        self.cursor = self.buffer.len();
        self.text_selection.clear();
        self.widgets.handle(crate::actions::Action::Escape);
        self.resume_completion();
        self.note_activity();
        self.request_completion();
    }
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
    pub(crate) fn take_draft(&mut self) -> Option<Draft> {
        self.draft.take()
    }
}
