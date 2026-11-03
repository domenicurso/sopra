use crate::editor::{ExitReason, RunResult};

pub(crate) fn write_result(
    result: &RunResult,
    output: &mut impl std::io::Write,
    prompts: (&str, &str),
    styled_transcript: bool,
) -> std::io::Result<()> {
    let action = match result.reason {
        ExitReason::Accepted => "accept",
        ExitReason::Interrupted => "interrupt",
        ExitReason::DelegateTab => "tab",
        ExitReason::DelegateEof => "eof",
        ExitReason::ChangeDirectory => "directory",
    };
    let display_text = result
        .submitted_spans
        .iter()
        .map(|span| span.text.as_str())
        .collect::<String>();
    write!(
        output,
        "K2\t{action}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        result.cursor,
        nonempty_hex(result.buffer.as_bytes()),
        match encode_highlights(&result.buffer, &result.highlights) {
            value if value.is_empty() => "-".to_string(),
            value => value,
        },
        result.directory.as_ref().map_or_else(
            || "-".to_string(),
            |path| {
                use std::os::unix::ffi::OsStrExt;
                nonempty_hex(path.as_os_str().as_bytes())
            }
        ),
        nonempty_hex(display_text.as_bytes()),
        encode_display_highlights(&result.submitted_spans),
        nonempty_hex(prompts.0.as_bytes()),
        nonempty_hex(prompts.1.as_bytes())
    )?;
    if styled_transcript {
        write!(
            output,
            "\t{}",
            nonempty_hex(crate::prompt::shell::encode(&result.submitted_spans).as_bytes())
        )?;
    }
    writeln!(output)
}

fn encode_highlights(buffer: &str, spans: &[crate::syntax::SyntaxSpan]) -> String {
    spans
        .iter()
        .filter_map(|span| {
            let style = crate::render::region_highlight_style(span.style())?;
            let start = buffer[..span.start].chars().count();
            let end = buffer[..span.end].chars().count();
            (start < end).then(|| format!("{start} {end} {style}"))
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for &byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn nonempty_hex(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        "-".to_string()
    } else {
        hex_encode(bytes)
    }
}

fn encode_display_highlights(spans: &[crate::prompt::PromptSpan]) -> String {
    let mut offset = 0;
    let mut highlights = Vec::new();
    for span in spans {
        let end = offset + span.text.chars().count();
        if let Some(style) = crate::render::region_highlight_style(span.style)
            && offset < end
        {
            highlights.push(format!("{offset} {end} {style}"));
        }
        offset = end;
    }
    if highlights.is_empty() {
        "-".to_string()
    } else {
        highlights.join(";")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt::PromptSpan;
    use ratatui::style::Style;

    #[test]
    fn styled_transcripts_are_negotiated_and_keep_prompt_attributes() -> std::io::Result<()> {
        let result = RunResult {
            reason: ExitReason::Accepted,
            buffer: "echo one\necho two".into(),
            cursor: 17,
            highlights: Vec::new(),
            directory: None,
            submitted_spans: vec![PromptSpan {
                text: "\n> %".into(),
                style: Style::new().dim(),
            }],
        };
        for (enabled, fields) in [(false, 10), (true, 11)] {
            let mut bytes = Vec::new();
            write_result(&result, &mut bytes, ("$ ", "> "), enabled)?;
            let reply = String::from_utf8(bytes).map_err(std::io::Error::other)?;
            assert_eq!(reply.trim_end().split('\t').count(), fields);
            if enabled {
                let styled = "%{\x1b[2m%}\n> %%%{\x1b[0m%}";
                assert!(reply.trim_end().ends_with(&hex_encode(styled.as_bytes())));
            }
        }
        Ok(())
    }
}
