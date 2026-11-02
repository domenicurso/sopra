use super::canvas::{Canvas, TextRun};
use ratatui::style::{Color, Modifier, Style};
use sopra_completion::CompletionItem;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub(super) fn paint(canvas: &mut Canvas, item: &CompletionItem, query: &str, run: TextRun) {
    let (column, row) = run.position;
    let (base, max_width) = (run.style, run.max_width);
    let fallback = fuzzy_positions(&item.display, query);
    let truncated = UnicodeWidthStr::width(item.display.as_str()) > usize::from(max_width);
    let text_width = max_width.saturating_sub(u16::from(truncated));
    let mut offset = 0_u16;
    for (index, (byte_offset, grapheme)) in item.display.grapheme_indices(true).enumerate() {
        let width = UnicodeWidthStr::width(grapheme) as u16;
        if width == 0 {
            continue;
        }
        if offset.saturating_add(width) > text_width {
            break;
        }
        let matched = if item.match_indices.is_empty() {
            fallback.get(index).copied().unwrap_or(false)
        } else {
            item.match_indices.contains(&byte_offset)
        };
        canvas.text(TextRun {
            position: (column + offset, row),
            text: grapheme,
            style: matched_style(base, matched),
            max_width: max_width.saturating_sub(offset),
        });
        offset = offset.saturating_add(width);
    }
    if truncated && max_width > 0 {
        canvas.put(column + offset, row, "…", base);
    }
}

fn matched_style(base: Style, matched: bool) -> Style {
    if matched {
        base.fg(Color::Green)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        base
    }
}

fn fuzzy_positions(text: &str, query: &str) -> Vec<bool> {
    let graphemes = text.graphemes(true).collect::<Vec<_>>();
    let mut matched = vec![false; graphemes.len()];
    let mut search_start = 0;
    for wanted in query
        .rsplit('/')
        .next()
        .unwrap_or(query)
        .graphemes(true)
        .filter(|value| !value.chars().all(char::is_whitespace))
    {
        let Some(offset) = graphemes[search_start..]
            .iter()
            .position(|value| value.eq_ignore_ascii_case(wanted))
            .map(|index| index + search_start)
        else {
            continue;
        };
        matched[offset] = true;
        search_start = offset + 1;
    }
    matched
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;
    use sopra_completion::CompletionKind;

    #[test]
    fn truncation_preserves_graphemes_and_reserves_space_for_the_ellipsis() {
        for (label, width, expected) in [
            ("abcdef", 4, "abc…"),
            ("a界bc", 3, "a… "),
            ("界abc", 1, "…"),
            ("e\u{301}long", 2, "e\u{301}…"),
            ("abcd", 4, "abcd"),
            ("long", 0, ""),
        ] {
            let mut canvas = Canvas::new(Rect::new(0, 0, width, 1));
            let item = CompletionItem::new(label, "", label, CompletionKind::Generic);
            paint(
                &mut canvas,
                &item,
                "",
                TextRun {
                    position: (0, 0),
                    text: label,
                    style: Style::default(),
                    max_width: width,
                },
            );
            let text: String = canvas
                .buffer
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect();
            assert_eq!(text, expected, "label: {label}, width: {width}");
        }
    }

    #[test]
    fn empty_queries_do_not_highlight_completion_labels() {
        assert!(
            fuzzy_positions("private", "")
                .iter()
                .all(|matched| !matched)
        );
    }
}
