use ratatui::style::{Color, Modifier, Style};

pub(super) fn style_tag(tag: &str) -> Option<Style> {
    let style = Style::default();
    Some(match tag {
        "reverse" => style.add_modifier(Modifier::REVERSED),
        "default" => style.fg(Color::Reset),
        "dim" => style.add_modifier(Modifier::DIM),
        "bold" => style.add_modifier(Modifier::BOLD),
        "italic" => style.add_modifier(Modifier::ITALIC),
        "underline" => style.add_modifier(Modifier::UNDERLINED),
        "black" => style.fg(Color::Black),
        "red" => style.fg(Color::Red),
        "green" => style.fg(Color::Green),
        "yellow" => style.fg(Color::Yellow),
        "blue" => style.fg(Color::Blue),
        "magenta" => style.fg(Color::Magenta),
        "cyan" => style.fg(Color::Cyan),
        "white" => style.fg(Color::White),
        _ => return None,
    })
}

pub(super) fn apply(input: &str, style: &mut Style, stack: &mut Vec<Style>) -> Option<usize> {
    let end = input.find(']')?;
    let tag = input.get(1..end)?;
    if tag == "/" {
        *style = stack.pop()?;
    } else {
        let overlay = style_tag(tag)?;
        stack.push(*style);
        *style = style.patch(overlay);
    }
    Some(end + 1)
}

#[cfg(test)]
mod tests {
    use crate::prompt::parse;
    use ratatui::style::{Color, Modifier};

    #[test]
    fn nested_tags_restore_outer_and_ansi_styles() {
        let spans = parse("\x1b[31mred[bold]bold[dim]:[/]bold[/]red\x1b[0m");
        assert_eq!(
            spans
                .iter()
                .map(|span| span.text.as_str())
                .collect::<String>(),
            "redbold:boldred"
        );
        assert!(spans.iter().all(|span| span.style.fg == Some(Color::Red)));
        assert!(
            spans[2]
                .style
                .add_modifier
                .contains(Modifier::BOLD | Modifier::DIM)
        );
        assert!(!spans[3].style.add_modifier.contains(Modifier::DIM));
        assert!(!spans[4].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn escaped_and_unknown_tags_are_literal() {
        let spans = parse("[[dim] [unknown] [/]");
        assert_eq!(spans[0].text, "[dim] [unknown] [/]");
    }
}
