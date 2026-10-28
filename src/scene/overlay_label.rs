use super::canvas::{Canvas, TextRun};
use ratatui::style::{Color, Modifier, Style};
use sopra_completion::CompletionItem;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub(super) fn paint(canvas: &mut Canvas, item: &CompletionItem, query: &str, run: TextRun) {
    let (column, row) = run.position;
    let (base, max_width) = (run.style, run.max_width);
    let fallback = fuzzy_positions(&item.display, query);
    let mut offset = 0_u16;
    for (index, (byte_offset, grapheme)) in item.display.grapheme_indices(true).enumerate() {
        let width = UnicodeWidthStr::width(grapheme) as u16;
        if width == 0 {
            continue;
        }
        if offset.saturating_add(width) > max_width {
            if max_width > 0 {
                canvas.put(column + max_width - 1, row, "…", base);
            }
            return;
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
    use super::fuzzy_positions;

    #[test]
    fn empty_queries_do_not_highlight_completion_labels() {
        assert!(
            fuzzy_positions("private", "")
                .iter()
                .all(|matched| !matched)
        );
    }
}
