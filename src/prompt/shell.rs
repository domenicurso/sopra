use super::{PromptSpan, template::Template};
use std::{collections::BTreeMap, io};

pub(crate) fn render(source: &str) -> io::Result<String> {
    Ok(encode(&Template::compile(source)?.render(&BTreeMap::new())))
}

pub(crate) fn encode(spans: &[PromptSpan]) -> String {
    let mut output = String::new();
    for span in spans {
        let bytes = crate::render::style_bytes(span.style);
        if !bytes.is_empty() {
            output.push_str(&format!("%{{{}%}}", String::from_utf8_lossy(&bytes)));
        }
        output.push_str(&span.text.replace('%', "%%"));
        output.push_str("%{\x1b[0m%}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shell_prompt_escapes_percent_and_marks_nonprinting_styles() -> io::Result<()> {
        let result = render("[dim]$[/dim] 100% ")?;
        assert!(result.contains("%{\x1b[2m%}"));
        assert!(result.contains("100%%"));
        assert!(!result.contains("[dim]"));
        Ok(())
    }
}
