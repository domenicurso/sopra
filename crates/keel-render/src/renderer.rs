use keel_core::{
    EditorSnapshot, SpanStyle, StyledSpan, SurfaceFrame, SurfaceLine, TerminalSize,
};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Debug, Default)]
pub struct Renderer;

impl Renderer {
    pub fn render_active(
        &self,
        prompt_left: &str,
        prompt_right: &str,
        editor: &EditorSnapshot,
        terminal_size: TerminalSize,
    ) -> SurfaceFrame {
        self.render_active_with_cursor(
            prompt_left,
            prompt_right,
            editor,
            terminal_size,
            u8::MAX,
        )
    }

    pub fn render_active_with_cursor(
        &self,
        prompt_left: &str,
        prompt_right: &str,
        editor: &EditorSnapshot,
        terminal_size: TerminalSize,
        cursor_alpha: u8,
    ) -> SurfaceFrame {
        self.render(prompt_left, prompt_right, editor, terminal_size, cursor_alpha)
    }

    pub fn render_submitted(
        &self,
        prompt_left: &str,
        editor: &EditorSnapshot,
        terminal_size: TerminalSize,
    ) -> SurfaceFrame {
        self.render(prompt_left, "", editor, terminal_size, 0)
    }

    fn render(
        &self,
        prompt_left: &str,
        prompt_right: &str,
        editor: &EditorSnapshot,
        terminal_size: TerminalSize,
        cursor_alpha: u8,
    ) -> SurfaceFrame {
        let columns = terminal_size.columns.max(1) as usize;
        let mut frame = SurfaceFrame {
            lines: vec![SurfaceLine::default()],
            cursor: None,
        };
        let mut row = 0usize;
        let mut column = 0usize;

        self.push_text(
            &mut frame,
            prompt_left,
            SpanStyle::Prompt,
            columns,
            &mut row,
            &mut column,
        );

        let buffer_graphemes = UnicodeSegmentation::graphemes(editor.buffer.as_str(), true)
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let cursor_index = editor.cursor.min(buffer_graphemes.len());
        let cursor_visible = cursor_alpha > 0;

        for (index, grapheme) in buffer_graphemes.iter().enumerate() {
            let style = if cursor_visible && index == cursor_index {
                SpanStyle::CursorBlock {
                    alpha: cursor_alpha,
                }
            } else {
                SpanStyle::Plain
            };
            self.push_grapheme(
                &mut frame,
                grapheme,
                style,
                columns,
                &mut row,
                &mut column,
            );
        }

        if cursor_visible && cursor_index == buffer_graphemes.len() {
            self.push_grapheme(
                &mut frame,
                " ",
                SpanStyle::CursorBlock {
                    alpha: cursor_alpha,
                },
                columns,
                &mut row,
                &mut column,
            );
        }

        if !prompt_right.is_empty() {
            self.append_right_prompt(&mut frame, prompt_right, columns);
        }

        frame
    }

    fn append_right_prompt(&self, frame: &mut SurfaceFrame, prompt_right: &str, columns: usize) {
        if frame.lines.len() != 1 {
            return;
        }

        let line = &mut frame.lines[0];
        let current_width = visual_width_of_line(line);
        let right_width = prompt_right.width();
        let Some((left_capacity, right_origin)) = right_prompt_region(columns, right_width) else {
            return;
        };

        if current_width >= columns || current_width > left_capacity {
            return;
        }

        let padding = right_origin.saturating_sub(current_width);
        if padding > 0 {
            push_span(line, " ".repeat(padding), SpanStyle::Muted);
        }
        push_span(line, prompt_right.to_string(), SpanStyle::Muted);
    }

    fn push_text(
        &self,
        frame: &mut SurfaceFrame,
        text: &str,
        style: SpanStyle,
        columns: usize,
        row: &mut usize,
        column: &mut usize,
    ) {
        for grapheme in UnicodeSegmentation::graphemes(text, true) {
            self.push_grapheme(frame, grapheme, style, columns, row, column);
        }
    }

    fn push_grapheme(
        &self,
        frame: &mut SurfaceFrame,
        grapheme: &str,
        style: SpanStyle,
        columns: usize,
        row: &mut usize,
        column: &mut usize,
    ) {
        if grapheme == "\n" {
            frame.lines.push(SurfaceLine::default());
            *row += 1;
            *column = 0;
            return;
        }

        let width = grapheme_width(grapheme);
        if *column > 0 && *column + width > columns {
            frame.lines.push(SurfaceLine::default());
            *row += 1;
            *column = 0;
        }

        push_span(
            frame
                .lines
                .last_mut()
                .expect("surface frame always contains a line"),
            grapheme.to_string(),
            style,
        );
        *column += width;
    }
}

fn push_span(line: &mut SurfaceLine, text: String, style: SpanStyle) {
    if text.is_empty() {
        return;
    }

    if let Some(previous) = line.spans.last_mut() {
        if previous.style == style {
            previous.text.push_str(&text);
            return;
        }
    }

    line.spans.push(StyledSpan { text, style });
}

fn visual_width_of_line(line: &SurfaceLine) -> usize {
    line.spans
        .iter()
        .map(|span| span.text.width())
        .sum::<usize>()
}

fn grapheme_width(grapheme: &str) -> usize {
    if grapheme.chars().count() == 1 {
        return UnicodeWidthChar::width(grapheme.chars().next().unwrap())
            .unwrap_or(0)
            .max(1);
    }

    UnicodeWidthStr::width(grapheme).max(1)
}

fn right_prompt_region(columns: usize, right_width: usize) -> Option<(usize, usize)> {
    let total_width = u16::try_from(columns).ok()?;
    let right_width = u16::try_from(right_width).ok()?;

    if right_width == 0 || right_width >= total_width {
        return None;
    }

    let regions = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(right_width)])
        .split(Rect::new(0, 0, total_width, 1));
    let left = regions[0];
    let right = regions[1];

    Some((left.width as usize, right.x as usize))
}

#[cfg(test)]
mod tests {
    use super::Renderer;
    use keel_core::{EditorSnapshot, SpanStyle, TerminalSize};

    #[test]
    fn renders_prompt_and_cursor_position() {
        let renderer = Renderer;
        let frame = renderer.render_active(
            "keel> ",
            "",
            &EditorSnapshot {
                buffer: "echo hello".to_string(),
                cursor: 10,
                selection: None,
            },
            TerminalSize {
                columns: 32,
                rows: 24,
            },
        );

        assert_eq!(frame.lines[0].spans[0].style, SpanStyle::Prompt);
        assert_eq!(frame.plain_text(), "keel> echo hello ");
        assert!(frame.cursor.is_none());
    }

    #[test]
    fn wraps_long_input_to_next_line() {
        let renderer = Renderer;
        let frame = renderer.render_active(
            "keel> ",
            "",
            &EditorSnapshot {
                buffer: "abcdefghijklmnop".to_string(),
                cursor: 16,
                selection: None,
            },
            TerminalSize {
                columns: 12,
                rows: 24,
            },
        );

        assert_eq!(frame.lines.len(), 2);
        assert_eq!(frame.plain_text(), "keel> abcdef\nghijklmnop ");
    }

    #[test]
    fn appends_right_prompt_when_line_has_room() {
        let renderer = Renderer;
        let frame = renderer.render_active(
            "keel> ",
            "[0]",
            &EditorSnapshot {
                buffer: "ls".to_string(),
                cursor: 2,
                selection: None,
            },
            TerminalSize {
                columns: 16,
                rows: 24,
            },
        );

        assert_eq!(frame.plain_text(), "keel> ls     [0]");
    }

    #[test]
    fn treats_emoji_clusters_as_single_cursor_step() {
        let renderer = Renderer;
        let frame = renderer.render_active(
            "keel> ",
            "",
            &EditorSnapshot {
                buffer: "👍🏽x".to_string(),
                cursor: 1,
                selection: None,
            },
            TerminalSize {
                columns: 32,
                rows: 24,
            },
        );

        assert_eq!(frame.plain_text(), "keel> 👍🏽x");
        assert_eq!(
            frame.lines[0].spans[2].style,
            SpanStyle::CursorBlock { alpha: u8::MAX }
        );
    }
}
