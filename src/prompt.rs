use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptSpan {
    pub(crate) text: String,
    pub(crate) style: Style,
}

pub(crate) fn parse(prompt: &str) -> Vec<PromptSpan> {
    let mut spans = Vec::new();
    let mut style = Style::default();
    let mut text = String::new();
    let bytes = prompt.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == 0x1b {
            push_span(&mut spans, &mut text, style);
            index = consume_escape(prompt, index, &mut style);
            continue;
        }
        let character = prompt[index..].chars().next().unwrap_or_default();
        if !character.is_control() {
            text.push(character);
        }
        index += character.len_utf8();
    }
    push_span(&mut spans, &mut text, style);
    spans
}

pub(crate) fn width(spans: &[PromptSpan]) -> usize {
    spans
        .iter()
        .map(|span| unicode_width::UnicodeWidthStr::width(span.text.as_str()))
        .sum()
}

fn push_span(spans: &mut Vec<PromptSpan>, text: &mut String, style: Style) {
    if !text.is_empty() {
        spans.push(PromptSpan {
            text: std::mem::take(text),
            style,
        });
    }
}

fn consume_escape(prompt: &str, start: usize, style: &mut Style) -> usize {
    let bytes = prompt.as_bytes();
    if bytes.get(start + 1) == Some(&b'[') {
        let end = find_escape_end(bytes, start + 2);
        if end > start + 2 && bytes[end - 1] == b'm' {
            apply_sgr(style, &prompt[start + 2..end - 1]);
        }
        return end;
    }
    if bytes.get(start + 1) == Some(&b']') {
        return consume_osc(bytes, start + 2);
    }
    (start + 2).min(bytes.len())
}

fn find_escape_end(bytes: &[u8], start: usize) -> usize {
    bytes
        .iter()
        .enumerate()
        .skip(start)
        .find(|(_, byte)| (0x40..=0x7e).contains(&**byte))
        .map_or(bytes.len(), |(index, _)| index + 1)
}

fn consume_osc(bytes: &[u8], start: usize) -> usize {
    let mut index = start;
    while index < bytes.len() {
        if bytes[index] == 0x07 {
            return index + 1;
        }
        if bytes[index] == 0x1b && bytes.get(index + 1) == Some(&b'\\') {
            return index + 2;
        }
        index += 1;
    }
    bytes.len()
}

fn apply_sgr(style: &mut Style, value: &str) {
    let params = value
        .split(';')
        .map(|part| part.parse::<u16>().unwrap_or(0))
        .collect::<Vec<_>>();
    let mut index = 0;
    while index < params.len() {
        let code = params[index];
        match code {
            0 => *style = Style::default(),
            1 => style.add_modifier = style.add_modifier.union(Modifier::BOLD),
            2 => style.add_modifier = style.add_modifier.union(Modifier::DIM),
            3 => style.add_modifier = style.add_modifier.union(Modifier::ITALIC),
            4 => style.add_modifier = style.add_modifier.union(Modifier::UNDERLINED),
            22 => *style = style.remove_modifier(Modifier::BOLD | Modifier::DIM),
            23 => *style = style.remove_modifier(Modifier::ITALIC),
            24 => *style = style.remove_modifier(Modifier::UNDERLINED),
            30..=37 | 90..=97 => style.fg = ansi_color(code),
            39 => style.fg = None,
            40..=47 | 100..=107 => style.bg = ansi_color(code - 10),
            49 => style.bg = None,
            38 | 48 => {
                let (color, consumed) = extended_color(&params[index + 1..]);
                if let Some(color) = color {
                    if code == 38 {
                        style.fg = Some(color);
                    } else {
                        style.bg = Some(color);
                    }
                }
                index += consumed;
            }
            _ => {}
        }
        index += 1;
    }
}

fn extended_color(params: &[u16]) -> (Option<Color>, usize) {
    match params {
        [5, value, ..] => (Some(Color::Indexed((*value).min(255) as u8)), 2),
        [2, red, green, blue, ..] => (
            Some(Color::Rgb(
                (*red).min(255) as u8,
                (*green).min(255) as u8,
                (*blue).min(255) as u8,
            )),
            4,
        ),
        _ => (None, 0),
    }
}

fn ansi_color(code: u16) -> Option<Color> {
    Some(match code {
        30 | 40 => Color::Black,
        31 | 41 => Color::Red,
        32 | 42 => Color::Green,
        33 | 43 => Color::Yellow,
        34 | 44 => Color::Blue,
        35 | 45 => Color::Magenta,
        36 | 46 => Color::Cyan,
        37 | 47 => Color::White,
        90 | 100 => Color::DarkGray,
        91 | 101 => Color::LightRed,
        92 | 102 => Color::LightGreen,
        93 | 103 => Color::LightYellow,
        94 | 104 => Color::LightBlue,
        95 | 105 => Color::LightMagenta,
        96 | 106 => Color::LightCyan,
        97 | 107 => Color::White,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::{parse, width};
    use ratatui::style::{Color, Modifier};

    #[test]
    fn strips_terminal_sequences_from_prompt_width() {
        let spans = parse("\x1b[32muser\x1b[0m in ~/repo $ ");
        assert_eq!(width(&spans), 17);
        assert_eq!(spans[0].style.fg, Some(Color::Green));
        assert_eq!(spans[1].style.fg, None);
    }

    #[test]
    fn keeps_prompt_modifiers() {
        let spans = parse("\x1b[1;4mprompt\x1b[0m");
        assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
        assert!(spans[0].style.add_modifier.contains(Modifier::UNDERLINED));
    }
}
