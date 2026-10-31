use ratatui::style::{Color, Modifier, Style};

pub(super) fn attributes(source: &str) -> Option<Style> {
    let mut result = Style::default();
    if source.is_empty() {
        return None;
    }
    for word in source.split_whitespace() {
        result = result.patch(attribute(word)?);
    }
    Some(result)
}

fn attribute(word: &str) -> Option<Style> {
    let style = Style::default();
    Some(match word {
        "bold" => style.add_modifier(Modifier::BOLD),
        "dim" => style.add_modifier(Modifier::DIM),
        "italic" => style.add_modifier(Modifier::ITALIC),
        "underline" => style.add_modifier(Modifier::UNDERLINED),
        "reverse" => style.add_modifier(Modifier::REVERSED),
        "default" => style.fg(Color::Reset),
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
