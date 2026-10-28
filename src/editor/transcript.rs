use crate::{
    prompt::PromptSpan,
    syntax::{self, SyntaxSpan},
};

// The command sent to Zsh and the text left on screen have different contents:
// continuation prefixes are display text and must never reach the interpreter.
pub(super) fn submitted_spans(
    buffer: &str,
    syntax: &[SyntaxSpan],
    secondary: &str,
) -> Vec<PromptSpan> {
    let secondary = crate::prompt::parse(secondary);
    let mut spans = Vec::new();
    let mut offset = 0;
    for (index, line) in buffer.split('\n').enumerate() {
        if index > 0 {
            append(&mut spans, "\n", ratatui::style::Style::default());
            for span in &secondary {
                append(&mut spans, &span.text, span.style);
            }
            offset += 1;
        }
        for (byte, character) in line.char_indices() {
            let mut encoded = [0; 4];
            append(
                &mut spans,
                character.encode_utf8(&mut encoded),
                syntax::style_at(syntax, offset + byte),
            );
        }
        offset += line.len();
    }
    spans
}

fn append(spans: &mut Vec<PromptSpan>, text: &str, style: ratatui::style::Style) {
    if let Some(last) = spans.last_mut().filter(|last| last.style == style) {
        last.text.push_str(text);
    } else {
        spans.push(PromptSpan {
            text: text.to_string(),
            style,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::submitted_spans;
    use ratatui::style::Color;

    #[test]
    fn continuation_prefixes_have_their_own_style_and_preserve_unicode() {
        let spans = submitted_spans("echo café\nprintf hé", &[], "[blue]>[/] ");
        let text = spans
            .iter()
            .map(|span| span.text.as_str())
            .collect::<String>();
        assert_eq!(text, "echo café\n> printf hé");
        assert!(
            spans
                .iter()
                .any(|span| span.text == ">" && span.style.fg == Some(Color::Blue))
        );
    }
}
