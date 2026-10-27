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
