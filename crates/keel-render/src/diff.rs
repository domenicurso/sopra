use keel_core::{SurfaceFrame, SurfaceLine, VisualCursor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchLine {
    pub row: u16,
    pub line: SurfaceLine,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FramePatch {
    pub previous_height: u16,
    pub next_height: u16,
    pub lines: Vec<PatchLine>,
    pub cursor: Option<VisualCursor>,
}

impl FramePatch {
    pub fn between(previous: Option<&SurfaceFrame>, next: &SurfaceFrame) -> Self {
        let previous_lines = previous.map(|frame| frame.lines.as_slice()).unwrap_or(&[]);
        let next_lines = next.lines.as_slice();
        let max_height = previous_lines.len().max(next_lines.len());
        let mut lines = Vec::new();

        for index in 0..max_height {
            let previous_line = previous_lines.get(index);
            let next_line = next_lines.get(index).cloned().unwrap_or_default();

            if previous_line != Some(&next_line) {
                lines.push(PatchLine {
                    row: index as u16,
                    line: next_line,
                });
            }
        }

        Self {
            previous_height: previous_lines.len() as u16,
            next_height: next_lines.len() as u16,
            lines,
            cursor: next.cursor,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FramePatch;
    use keel_core::{SpanStyle, StyledSpan, SurfaceFrame, SurfaceLine, VisualCursor};

    #[test]
    fn diffs_only_changed_lines() {
        let previous = SurfaceFrame {
            lines: vec![
                SurfaceLine {
                    spans: vec![StyledSpan {
                        text: "keel> ls".to_string(),
                        style: SpanStyle::Prompt,
                    }],
                },
                SurfaceLine {
                    spans: vec![StyledSpan {
                        text: "old".to_string(),
                        style: SpanStyle::Plain,
                    }],
                },
            ],
            cursor: None,
        };
        let next = SurfaceFrame {
            lines: vec![
                previous.lines[0].clone(),
                SurfaceLine {
                    spans: vec![StyledSpan {
                        text: "new".to_string(),
                        style: SpanStyle::Plain,
                    }],
                },
            ],
            cursor: Some(VisualCursor {
                row: 1,
                column: 3,
                style: keel_core::CursorStyle::Beam,
                visible: true,
            }),
        };

        let patch = FramePatch::between(Some(&previous), &next);
        assert_eq!(patch.lines.len(), 1);
        assert_eq!(patch.lines[0].row, 1);
        assert_eq!(patch.cursor, next.cursor);
    }
}
