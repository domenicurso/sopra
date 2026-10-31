use super::output::Output;

pub(super) fn render(source: &str, output: &Output) -> String {
    if !output.color {
        return source.into();
    }
    let mut result = String::new();
    let mut cursor = 0;
    let mut key = true;
    let mut line_start = true;
    while cursor < source.len() {
        let remaining = &source[cursor..];
        let first = remaining.as_bytes()[0];
        let (length, style) = match first {
            b'\n' => {
                key = true;
                line_start = true;
                (1, "")
            }
            b' ' | b'\t' | b'\r' => (1, ""),
            b'#' => (remaining.find('\n').unwrap_or(remaining.len()), "2"),
            b'[' if line_start => (remaining.find('\n').unwrap_or(remaining.len()), "1;35"),
            b'\'' | b'"' => (quoted(remaining), if key { "36" } else { "32" }),
            b'=' => {
                key = false;
                (1, "2")
            }
            b'[' | b']' | b'{' | b'}' | b',' => (1, "2"),
            _ => {
                let length = remaining
                    .find(|c: char| c.is_whitespace() || "=[]{}#,\"'".contains(c))
                    .unwrap_or(remaining.len())
                    .max(1);
                (length, if key { "36" } else { "35" })
            }
        };
        let token = &source[cursor..cursor + length];
        if style.is_empty() {
            result.push_str(token);
        } else {
            result.push_str(&output.style(token, style));
        }
        if !token.trim().is_empty() {
            line_start = false;
        }
        cursor += length;
    }
    result
}

fn quoted(source: &str) -> usize {
    let quote = source.as_bytes()[0];
    let delimiter = if source.as_bytes().starts_with(&[quote; 3]) {
        3
    } else {
        1
    };
    let mut cursor = delimiter;
    while cursor < source.len() {
        if quote == b'"' && source.as_bytes()[cursor] == b'\\' {
            cursor += 1;
            if cursor < source.len() {
                cursor += source[cursor..].chars().next().map_or(0, char::len_utf8);
            }
        } else if source.as_bytes()[cursor..].starts_with(&[quote; 3][..delimiter]) {
            return cursor + delimiter;
        } else {
            cursor += source[cursor..].chars().next().map_or(0, char::len_utf8);
        }
    }
    source.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn highlighting_preserves_toml_including_unicode_and_multiline_strings() {
        let source = "# comment\n[prompt]\nleft = \"\"\"hello \\\"世界\n# still a string\"\"\"\n[editor]\nautosuggestions = true\n";
        let output = Output {
            width: 80,
            color: true,
            terminal: true,
        };
        let highlighted = render(source, &output);
        assert!(highlighted.contains("\x1b[1;35m[prompt]"));
        assert!(highlighted.contains("\x1b[36mleft"));
        assert!(highlighted.contains("\x1b[35mtrue"));
        assert!(highlighted.contains("# still a string"));
        let mut plain = String::new();
        let mut pieces = highlighted.split('\x1b');
        plain.push_str(pieces.next().unwrap_or_default());
        for piece in pieces {
            plain.push_str(piece.split_once('m').map_or(piece, |(_, text)| text));
        }
        assert_eq!(plain, source);
        assert_eq!(
            render(
                source,
                &Output {
                    color: false,
                    ..output
                }
            ),
            source
        );
    }
}
