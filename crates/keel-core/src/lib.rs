use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostSnapshot {
    pub buffer: String,
    pub cursor: usize,
    pub columns: u16,
    pub rows: u16,
    pub keymap: String,
}

impl Default for HostSnapshot {
    fn default() -> Self {
        Self {
            buffer: String::new(),
            cursor: 0,
            columns: 80,
            rows: 24,
            keymap: "main".to_string(),
        }
    }
}

impl HostSnapshot {
    pub fn sanitized(&self) -> Self {
        let cursor = self.cursor.min(self.buffer.chars().count());
        Self {
            buffer: self.buffer.clone(),
            cursor,
            columns: self.columns.max(1),
            rows: self.rows.max(1),
            keymap: if self.keymap.is_empty() {
                "main".to_string()
            } else {
                self.keymap.clone()
            },
        }
    }

    pub fn character_count(&self) -> usize {
        self.buffer.chars().count()
    }

    pub fn cursor_width(&self) -> usize {
        self.buffer
            .chars()
            .take(self.cursor.min(self.character_count()))
            .collect::<String>()
            .width()
    }
}

#[cfg(test)]
mod tests {
    use super::HostSnapshot;

    #[test]
    fn sanitizes_cursor_and_terminal_dimensions() {
        let snapshot = HostSnapshot {
            buffer: "hello".to_string(),
            cursor: 99,
            columns: 0,
            rows: 0,
            keymap: String::new(),
        }
        .sanitized();

        assert_eq!(snapshot.cursor, 5);
        assert_eq!(snapshot.columns, 1);
        assert_eq!(snapshot.rows, 1);
        assert_eq!(snapshot.keymap, "main");
    }

    #[test]
    fn measures_cursor_by_terminal_cells() {
        let snapshot = HostSnapshot {
            buffer: "a界b".to_string(),
            cursor: 2,
            ..HostSnapshot::default()
        };

        assert_eq!(snapshot.cursor_width(), 3);
    }
}
