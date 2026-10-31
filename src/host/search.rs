use super::rows::Row;

pub(super) fn matches(rows: &[Row], query: &str) -> Vec<usize> {
    let query = query.to_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    let mut matches = Vec::new();
    let mut start = 0;
    while start < rows.len() {
        let mut end = start + 1;
        while end < rows.len() && continuous(&rows[end - 1], &rows[end]) {
            end += 1;
        }
        let mut text = String::new();
        let mut boundaries = Vec::new();
        for (index, row) in rows.iter().enumerate().take(end).skip(start) {
            boundaries.push((text.len(), index));
            text.push_str(&row.text.to_lowercase());
        }
        for (offset, _) in text.match_indices(&query) {
            if let Some((_, row)) = boundaries.iter().rev().find(|(start, _)| *start <= offset) {
                matches.push(*row);
            }
        }
        start = end;
    }
    matches.dedup();
    matches
}
fn continuous(left: &Row, right: &Row) -> bool {
    left.block == right.block
        && match (&left.source, &right.source) {
            (Some((section, range)), Some((next_section, next))) => {
                section == next_section && range.end == next.start
            }
            _ => false,
        }
}

#[cfg(test)]
mod tests {
    use super::super::rows::Section;
    use super::*;
    #[test]
    fn finds_text_across_soft_wraps_but_not_across_lines_or_blocks() {
        let row = |text: &str, range| Row {
            block: 1,
            text: text.into(),
            source: Some((Section::Output, range)),
        };
        let rows = vec![
            row("hello ", 0..6),
            row("world", 6..11),
            row("again", 12..17),
        ];
        assert_eq!(matches(&rows, "LO WOR"), vec![0]);
        assert!(matches(&rows, "worldagain").is_empty());
    }
}
