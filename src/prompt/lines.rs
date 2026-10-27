use unicode_width::UnicodeWidthStr;

use super::PromptSpan;

pub(crate) fn widths(spans: &[PromptSpan]) -> Vec<usize> {
    let mut widths = vec![0];
    for span in spans {
        for (index, part) in span.text.split('\n').enumerate() {
            if index > 0 {
                widths.push(0);
            }
            if let Some(width) = widths.last_mut() {
                *width += UnicodeWidthStr::width(part);
            }
        }
    }
    widths
}

#[cfg(test)]
mod tests {
    #[test]
    fn widths_preserve_styles_across_line_breaks() {
        let spans = super::super::parse("[cyan]first\n$ [/]");
        assert_eq!(super::widths(&spans), [5, 2]);
    }
}
