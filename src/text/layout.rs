use super::flow::TextFlow;
use crate::{input::CursorPosition, prompt::PromptSpan};
use std::{cell::RefCell, rc::Rc};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone)]
pub(crate) struct Glyph {
    pub text: String,
    pub position: CursorPosition,
    pub byte: Option<usize>,
    pub style: ratatui::style::Style,
}

pub(crate) struct Layout {
    pub prompt: Vec<Glyph>,
    pub command: Vec<Glyph>,
    pub positions: Vec<(usize, CursorPosition)>,
    pub start: CursorPosition,
    pub end: CursorPosition,
}

#[derive(Clone, PartialEq, Eq)]
struct Key {
    anchor: CursorPosition,
    columns: u16,
    prompt: Vec<PromptSpan>,
    secondary: Vec<PromptSpan>,
    buffer: String,
}

pub(crate) struct Input<'a> {
    pub anchor: CursorPosition,
    pub columns: u16,
    pub prompt: &'a [PromptSpan],
    pub secondary: &'a [PromptSpan],
    pub buffer: &'a str,
}

#[derive(Default)]
pub(crate) struct Cache(RefCell<Option<(Key, Rc<Layout>)>>);
impl Cache {
    pub(crate) fn get(&self, input: Input<'_>) -> Rc<Layout> {
        let key = Key {
            anchor: input.anchor,
            columns: input.columns,
            prompt: input.prompt.into(),
            secondary: input.secondary.into(),
            buffer: input.buffer.into(),
        };
        let mut cache = self.0.borrow_mut();
        if let Some((_, layout)) = cache.as_ref().filter(|(old, _)| *old == key) {
            return layout.clone();
        }
        let layout = Rc::new(Layout::new(input));
        *cache = Some((key, layout.clone()));
        layout
    }
}

impl Layout {
    pub(crate) fn new(input: Input<'_>) -> Self {
        let mut flow = TextFlow::new(input.anchor, input.columns);
        let prompt = styled_glyphs(&mut flow, input.prompt);
        let start = flow.cursor();
        let mut command = Vec::new();
        let mut positions = Vec::new();
        for (byte, grapheme) in input.buffer.grapheme_indices(true) {
            if grapheme == "\n" {
                positions.push((byte, flow.cursor()));
                flow.advance(grapheme);
                command.extend(styled_glyphs(&mut flow, input.secondary));
            } else if let Some(position) = flow.advance(grapheme) {
                positions.push((byte, position));
                command.push(Glyph {
                    text: grapheme.into(),
                    byte: Some(byte),
                    position,
                    style: ratatui::style::Style::default(),
                });
            }
        }
        let end = flow.cursor();
        positions.push((input.buffer.len(), end));
        Self {
            prompt,
            command,
            positions,
            start,
            end,
        }
    }

    pub(crate) fn position(&self, byte: usize) -> CursorPosition {
        self.positions
            .iter()
            .rev()
            .find(|(offset, _)| *offset <= byte)
            .map_or(self.start, |(_, position)| *position)
    }
}

fn styled_glyphs(flow: &mut TextFlow, spans: &[PromptSpan]) -> Vec<Glyph> {
    let mut glyphs = Vec::new();
    for span in spans {
        for grapheme in span.text.graphemes(true) {
            if let Some(position) = flow.advance(grapheme) {
                glyphs.push(Glyph {
                    text: grapheme.into(),
                    position,
                    byte: None,
                    style: span.style,
                });
            }
        }
    }
    glyphs
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wrapping_unicode_and_continuations_share_the_same_position_map() {
        let prompt = crate::prompt::parse("P\n$ ");
        let secondary = crate::prompt::parse("[dim]> [/dim]");
        let layout = Layout::new(Input {
            anchor: CursorPosition { row: 0, column: 0 },
            columns: 6,
            prompt: &prompt,
            secondary: &secondary,
            buffer: "a界bc\nz",
        });
        assert_eq!(layout.position(5), CursorPosition { row: 2, column: 0 });
        assert_eq!(layout.end, CursorPosition { row: 3, column: 3 });
        for glyph in layout.command.iter().filter(|glyph| glyph.byte.is_some()) {
            assert_eq!(
                layout.position(glyph.byte.unwrap_or_default()),
                glyph.position
            );
        }
    }
}
