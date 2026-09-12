use ratatui::{style::Style, text::Span};
use unicode_segmentation::UnicodeSegmentation;

pub(crate) fn match_spans(
    text: &str,
    query: &str,
    match_indices: &[usize],
    base: Style,
    matching: Style,
) -> Vec<Span<'static>> {
    let graphemes = text.graphemes(true).collect::<Vec<_>>();
    let matched = if !match_indices.is_empty() {
        text.grapheme_indices(true)
            .map(|(offset, grapheme)| {
                let end = offset + grapheme.len();
                match_indices
                    .iter()
                    .any(|index| (*index >= offset) && (*index < end))
            })
            .collect::<Vec<_>>()
    } else {
        fuzzy_positions(&graphemes, query)
    };

    let mut spans = Vec::new();
    let mut current = String::new();
    let mut current_style = base;
    for (index, grapheme) in graphemes.iter().enumerate() {
        let style = if matched[index] { matching } else { base };
        if style != current_style && !current.is_empty() {
            spans.push(Span::styled(std::mem::take(&mut current), current_style));
        }
        current_style = style;
        current.push_str(grapheme);
    }
    if !current.is_empty() {
        spans.push(Span::styled(current, current_style));
    }
    spans
}

fn fuzzy_positions(graphemes: &[&str], query: &str) -> Vec<bool> {
    let mut matched = vec![false; graphemes.len()];
    let mut search_start = 0;
    for wanted in query
        .graphemes(true)
        .filter(|grapheme| !grapheme.chars().all(char::is_whitespace))
    {
        let Some(index) = graphemes[search_start..]
            .iter()
            .position(|grapheme| grapheme.eq_ignore_ascii_case(wanted))
            .map(|index| index + search_start)
        else {
            continue;
        };
        matched[index] = true;
        search_start = index + 1;
    }
    matched
}
