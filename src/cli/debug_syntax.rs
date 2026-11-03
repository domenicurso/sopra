use std::path::Path;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::output::Output;

const HEADERS: [&str; 4] = ["Bytes", "Kind", "Color", "Text"];

pub(super) fn render(source: &str, cwd: &Path, output: &Output) -> String {
    let rows = crate::syntax::highlight_without_cursor(source, cwd)
        .into_iter()
        .map(|span| {
            [
                format!("{}..{}", span.start, span.end),
                format!("{:?}", span.kind),
                format!(
                    "{:?}",
                    span.style().fg.unwrap_or(ratatui::style::Color::Reset)
                ),
                format!("{:?}", &source[span.start..span.end]),
            ]
        })
        .collect::<Vec<_>>();
    if !output.terminal {
        return std::iter::once(HEADERS.join("\t"))
            .chain(rows.iter().map(|row| row.join("\t")))
            .map(|row| format!("{row}\n"))
            .collect();
    }
    render_terminal(&rows, output.width.saturating_sub(1).max(1))
}

fn render_terminal(rows: &[[String; 4]], width: usize) -> String {
    let widths: [usize; 3] = std::array::from_fn(|column| {
        rows.iter()
            .map(|row| row[column].width())
            .chain(std::iter::once(HEADERS[column].width()))
            .max()
            .unwrap_or(0)
    });
    let prefix = widths.iter().sum::<usize>() + 6;
    let mut result = String::new();
    if prefix + HEADERS[3].width() > width {
        return render_stacked(rows, width);
    }
    let header = HEADERS.map(String::from);
    for row in std::iter::once(&header).chain(rows.iter()) {
        let label = (0..3)
            .map(|column| {
                format!(
                    "{}{}  ",
                    row[column],
                    " ".repeat(widths[column] - row[column].width())
                )
            })
            .collect::<String>();
        for (index, line) in wrap_cells(&row[3], width - prefix).iter().enumerate() {
            let indent = if index == 0 {
                label.clone()
            } else {
                " ".repeat(prefix)
            };
            result.push_str(&format!("{indent}{line}\n"));
        }
    }
    result
}

fn render_stacked(rows: &[[String; 4]], width: usize) -> String {
    let mut result = String::new();
    for row in rows {
        for (header, value) in HEADERS.iter().zip(row) {
            for line in wrap_cells(&format!("{header}: {value}"), width) {
                result.push_str(&format!("{line}\n"));
            }
        }
        result.push('\n');
    }
    result
}

fn wrap_cells(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut used = 0;
    for grapheme in text.graphemes(true) {
        let cells = grapheme.width();
        if used + cells > width && !line.is_empty() {
            lines.push(std::mem::take(&mut line));
            used = 0;
        }
        line.push_str(grapheme);
        used += cells;
    }
    lines.push(line);
    lines
}

#[cfg(test)]
#[path = "debug_syntax_tests.rs"]
mod tests;
