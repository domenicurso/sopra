use super::{Piece, Template};
use crate::prompt::PromptSpan;
use ratatui::style::{Modifier, Style};
use sopra::Text;

impl Template {
    pub(crate) fn render(
        &self,
        values: &std::collections::BTreeMap<String, Text>,
    ) -> Vec<PromptSpan> {
        let references = self
            .pieces
            .iter()
            .enumerate()
            .filter_map(|(i, piece)| {
                if let Piece::Value { name, .. } = piece {
                    Some((i, name))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let Some((first, _)) = references.first() else {
            return text(&self.pieces);
        };
        let last = references.last().map_or(*first, |(i, _)| *i);
        let mut output = text(&self.pieces[..*first]);
        let mut previous = None;
        for (index, name) in &references {
            let Some(value) = values
                .get(*name)
                .filter(|value| value.parts.iter().any(|part| !part.text.is_empty()))
            else {
                continue;
            };
            if let Some(before) = previous {
                output.extend(self.connector(before, *index));
            }
            if let Piece::Value { style, .. } = &self.pieces[*index] {
                output.extend(spans(value, *style));
            }
            previous = Some(*index);
        }
        output.extend(text(&self.pieces[last + 1..]));
        output
    }

    fn connector(&self, before: usize, after: usize) -> Vec<PromptSpan> {
        // A missing middle value leaves one connector. The first connector wins
        // when different literal connectors surround the omitted values.
        let end = self.pieces[before + 1..after]
            .iter()
            .position(|piece| matches!(piece, Piece::Value { .. }))
            .map_or(after, |offset| before + 1 + offset);
        text(&self.pieces[before + 1..end])
    }
}

fn text(pieces: &[Piece]) -> Vec<PromptSpan> {
    pieces
        .iter()
        .flat_map(|piece| match piece {
            Piece::Text(span) => crate::prompt::parse_text(&span.text)
                .into_iter()
                .map(|mut parsed| {
                    parsed.style = span.style.patch(parsed.style);
                    parsed
                })
                .collect(),
            _ => Vec::new(),
        })
        .collect()
}

fn spans(text: &Text, style: Style) -> Vec<PromptSpan> {
    text.parts
        .iter()
        .filter(|part| !part.text.is_empty())
        .map(|part| {
            let style = match part.emphasis {
                sopra::Emphasis::Selected => style.add_modifier(Modifier::REVERSED),
                sopra::Emphasis::Dim => style.add_modifier(Modifier::DIM),
                sopra::Emphasis::Bold => style.add_modifier(Modifier::BOLD),
                sopra::Emphasis::Normal => style,
            };
            PromptSpan {
                text: part.text.clone(),
                style,
            }
        })
        .collect()
}
