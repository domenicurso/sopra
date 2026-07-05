use keel_core::{CanvasBuffer, CellStyle, FrontendScene, PromptSpanStyle, PromptSurface, TerminalPoint};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{
    buffer::{WriteCursor, write_grapheme, write_text},
    cursor::visual_cursor,
    layout::right_prompt_regions,
};

#[derive(Debug, Default)]
pub struct Renderer;

impl Renderer {
    pub fn compose(&self, scene: &FrontendScene) -> CanvasBuffer {
        let mut buffer = CanvasBuffer::blank(scene.terminal_size);
        let mut write_cursor = WriteCursor::at(TerminalPoint { row: 0, column: 0 });

        write_prompt_surface(&mut buffer, &mut write_cursor, &scene.prompt_left);
        let prompt_end = write_cursor.point();

        let buffer_graphemes = UnicodeSegmentation::graphemes(scene.editor.buffer.as_str(), true)
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let cursor_index = scene.editor.cursor.min(buffer_graphemes.len());
        let mut visual_cursor_point = if cursor_index == 0 {
            Some(prompt_end)
        } else {
            None
        };

        for (index, grapheme) in buffer_graphemes.iter().enumerate() {
            if index == cursor_index {
                visual_cursor_point = Some(write_cursor.point());
            }

            write_grapheme(&mut buffer, &mut write_cursor, grapheme, CellStyle::Plain);
        }

        if visual_cursor_point.is_none() {
            visual_cursor_point = Some(write_cursor.point());
        }

        self.compose_right_prompt(&mut buffer, scene);

        buffer.cursor = visual_cursor_point.and_then(|point| {
            visual_cursor(point.row, point.column, scene.cursor)
        });

        buffer
    }

    fn compose_right_prompt(&self, buffer: &mut CanvasBuffer, scene: &FrontendScene) {
        if scene.prompt_right.is_empty() {
            return;
        }

        let current_width = scene.prompt_left.plain_text().width() + scene.editor.buffer.width();
        let right_width = scene.prompt_right.plain_text().width();
        let Some(regions) = right_prompt_regions(scene.terminal_size, right_width) else {
            return;
        };

        if current_width >= scene.terminal_size.columns as usize
            || current_width > regions.left_capacity
        {
            return;
        }

        let mut cursor = WriteCursor {
            row: 0,
            column: regions.right_origin as u16,
        };
        write_prompt_surface(buffer, &mut cursor, &scene.prompt_right);
    }
}

fn write_prompt_surface(
    buffer: &mut CanvasBuffer,
    cursor: &mut WriteCursor,
    surface: &PromptSurface,
) {
    for span in &surface.spans {
        write_text(buffer, cursor, &span.text, map_prompt_style(span.style));
    }
}

fn map_prompt_style(style: PromptSpanStyle) -> CellStyle {
    match style {
        PromptSpanStyle::Plain => CellStyle::Plain,
        PromptSpanStyle::Prompt => CellStyle::Prompt,
        PromptSpanStyle::Muted => CellStyle::Muted,
        PromptSpanStyle::Accent => CellStyle::Accent,
        PromptSpanStyle::StatusOk => CellStyle::StatusOk,
        PromptSpanStyle::StatusError => CellStyle::StatusError,
    }
}

#[cfg(test)]
mod tests {
    use super::Renderer;
    use keel_core::{
        CursorStyle, EditorSnapshot, FrontendScene, PromptSpan, PromptSpanStyle, PromptSurface,
        SceneCursor, TerminalSize,
    };

    #[test]
    fn composes_active_prompt_into_a_canvas() {
        let renderer = Renderer;
        let buffer = renderer.compose(&FrontendScene {
            terminal_size: TerminalSize {
                columns: 16,
                rows: 6,
            },
            prompt_left: PromptSurface {
                spans: vec![PromptSpan {
                    text: "keel> ".to_string(),
                    style: PromptSpanStyle::Prompt,
                }],
            },
            prompt_right: PromptSurface::default(),
            editor: EditorSnapshot {
                buffer: "echo hi".to_string(),
                cursor: 7,
                selection: None,
            },
            cursor: Some(SceneCursor {
                style: CursorStyle::Block,
                alpha: u8::MAX,
                visible: true,
            }),
            overlay_anchor: None,
        });

        assert_eq!(buffer.plain_text(), "keel> echo hi");
        assert_eq!(buffer.cursor.unwrap().column, 13);
    }

    #[test]
    fn wraps_editor_content_across_canvas_rows() {
        let renderer = Renderer;
        let buffer = renderer.compose(&FrontendScene {
            terminal_size: TerminalSize {
                columns: 12,
                rows: 6,
            },
            prompt_left: PromptSurface {
                spans: vec![PromptSpan {
                    text: "keel> ".to_string(),
                    style: PromptSpanStyle::Prompt,
                }],
            },
            prompt_right: PromptSurface::default(),
            editor: EditorSnapshot {
                buffer: "abcdefghijklmnop".to_string(),
                cursor: 16,
                selection: None,
            },
            cursor: None,
            overlay_anchor: None,
        });

        assert_eq!(buffer.content_height(), 2);
        assert_eq!(buffer.rows[0].plain_text(), "keel> abcdef");
        assert_eq!(buffer.rows[1].plain_text(), "ghijklmnop");
    }
}
