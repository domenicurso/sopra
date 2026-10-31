use super::output::{Output, wrap};

pub(super) fn render(source: &str, output: &Output) -> String {
    if !output.terminal {
        return source.into();
    }
    let rows = source
        .lines()
        .skip(1)
        .map(|line| line.splitn(3, '\t').collect::<Vec<_>>())
        .filter(|row| row.len() == 3)
        .collect::<Vec<_>>();
    let name_width = rows
        .iter()
        .map(|row| row[0].len())
        .max()
        .unwrap_or(6)
        .max(6);
    let status_width = rows
        .iter()
        .map(|row| row[2].len())
        .max()
        .unwrap_or(6)
        .max(6);
    let source_column = name_width + status_width + 6;
    let columns = source_column + 12 < output.width;
    let mut result = String::new();
    if columns {
        result.push_str(&output.style(
            &format!(
                "  {:name_width$}  {:status_width$}  SOURCE",
                "WIDGET", "STATUS"
            ),
            "1",
        ));
        result.push('\n');
    }
    for row in rows {
        result.push_str(&render_row(&row, output, name_width, status_width));
    }
    result
}

fn render_row(row: &[&str], output: &Output, name_width: usize, status_width: usize) -> String {
    let source_column = name_width + status_width + 6;
    let status_style = match row[2] {
        "loaded" => "32",
        "available" => "2",
        "loading" => "33",
        _ => "31",
    };
    let mut result = String::new();
    if source_column + 12 < output.width {
        let lines = wrap(row[1], output.width.saturating_sub(source_column + 1));
        for (index, line) in lines.iter().enumerate() {
            if index == 0 {
                result.push_str(&format!(
                    "  {}{}  {}{}  {}\n",
                    output.style(row[0], "1;36"),
                    " ".repeat(name_width - row[0].len()),
                    output.style(row[2], status_style),
                    " ".repeat(status_width - row[2].len()),
                    output.style(line, "2")
                ));
            } else {
                result.push_str(&format!(
                    "{}{}\n",
                    " ".repeat(source_column),
                    output.style(line, "2")
                ));
            }
        }
    } else {
        for (text, style) in [(row[0], "1;36"), (row[2], status_style), (row[1], "2")] {
            for line in wrap(text, output.width.saturating_sub(3)) {
                result.push_str(&format!("  {}\n", output.style(&line, style)));
            }
        }
        result.push('\n');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn widget_columns_align_and_narrow_views_preserve_long_paths_and_errors() {
        let source =
            "WIDGET\tSOURCE\tSTATUS\nclock\tbuilt-in\tavailable\nduration\tbuilt-in\tloaded\n";
        let output = Output {
            width: 80,
            color: false,
            terminal: true,
        };
        let table = render(source, &output);
        assert_eq!(
            table.lines().nth(1).and_then(|line| line.find("built-in")),
            table.lines().nth(2).and_then(|line| line.find("built-in"))
        );
        assert!(!table.contains('\t'));
        let source = format!(
            "{source}custom\t/a/very/long/widget/source/path.rs\terror: failed to compile widget source\n"
        );
        let table = render(
            &source,
            &Output {
                width: 24,
                ..output
            },
        );
        assert!(table.lines().all(|line| line.len() < 24));
        assert!(table.contains("error: failed"));
        assert!(table.contains("path.rs"));
        assert_eq!(
            render(
                &source,
                &Output {
                    terminal: false,
                    ..output
                }
            ),
            source
        );
    }
}
