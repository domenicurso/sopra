use super::{History, snapshot::Entry};
use crate::actions::Action;
use crate::{
    editor::{EditorConfig, EditorState},
    input::{CursorPosition, TerminalSize},
    scene::Scene,
};
use ratatui::style::Modifier;

fn editor(buffer: &str) -> EditorState {
    let mut editor = EditorState::new(EditorConfig {
        buffer: buffer.to_string(),
        cursor_chars: buffer.chars().count(),
        arm_completion: false,
        suppress_completion: false,
        completion_enabled: true,
        syntax_highlighting: false,
        cursor_animation: false,
        prompt: "$ ".to_string(),
        rprompt: "RIGHT".to_string(),
        anchor: CursorPosition { row: 0, column: 0 },
        size: TerminalSize::new(30, 12),
        cwd: std::path::PathBuf::from("."),
        palette: crate::palette::TerminalPalette::default(),
    });
    editor.set_history(History {
        entries: ["echo café 世界", "echo older"]
            .into_iter()
            .map(|command| Entry {
                command: command.to_string(),
                started: None,
            })
            .collect(),
        ..History::default()
    });
    editor
}

#[test]
fn suggestions_are_not_input_until_accepted_and_escape_dismisses_them() {
    let mut editor = editor("echo caz");
    assert!(editor.autosuggestion_spans().is_empty());
    editor = self::editor("echo caf");
    let spans = editor.autosuggestion_spans();
    assert_eq!(spans[0].text, "é 世界");
    assert!(spans[0].style.add_modifier.contains(Modifier::DIM));
    assert_eq!(editor.buffer(), "echo caf");
    editor.handle_key(Action::Escape);
    assert!(editor.autosuggestion_spans().is_empty());
    editor.handle_key(Action::Character('é'));
    assert!(!editor.autosuggestion_spans().is_empty());
    editor.handle_key(Action::Left);
    assert!(editor.autosuggestion_spans().is_empty());
}

#[test]
fn right_end_and_word_right_accept_history_without_splitting_unicode() {
    for key in [Action::Right, Action::End] {
        let mut editor = editor("echo caf");
        editor.handle_key(key);
        assert_eq!(editor.buffer(), "echo café 世界");
        assert_eq!(editor.cursor_byte(), editor.buffer().len());
    }
    let mut editor = editor("echo");
    editor.handle_key(Action::WordRight);
    assert_eq!(editor.buffer(), "echo café");
    editor.handle_key(Action::WordRight);
    assert_eq!(editor.buffer(), "echo café 世界");
}

#[test]
fn history_menu_replaces_the_entire_command_without_adding_a_space() {
    let mut editor = editor("echo");
    assert_eq!(editor.visible_items()[0].insert, "echo café 世界");
    editor.handle_key(Action::Tab);
    assert_eq!(editor.buffer(), "echo café 世界");
}

#[test]
fn ghost_text_clips_before_the_right_prompt_and_is_absent_from_transient_frames() {
    let mut editor = editor("echo");
    editor.hide_overlay();
    let frame = Scene::for_editor(
        &editor,
        TerminalSize::new(16, 12),
        std::time::Instant::now(),
    )
    .render();
    assert!(frame.buffer[(6, 0)].modifier.contains(Modifier::DIM));
    assert_eq!(frame.buffer[(11, 0)].symbol(), "R");
    assert_eq!(frame.required_height, 1);
    let transient = Scene::for_transient(&editor, TerminalSize::new(16, 12)).render();
    assert_eq!(transient.buffer[(7, 0)].symbol(), " ");
}

#[test]
fn disabling_inline_hints_keeps_history_completion_available() {
    let mut editor = editor("echo");
    editor.history.enabled = false;
    assert!(editor.autosuggestion_spans().is_empty());
    assert!(!editor.visible_items().is_empty());
    editor.handle_key(Action::Right);
    assert_eq!(editor.buffer(), "echo");
}

#[test]
fn configured_styling_does_not_interpret_history_as_markup() {
    let mut editor = editor("echo");
    editor.history.entries = vec![Entry {
        command: "echo [red]literal[/]".to_string(),
        started: None,
    }];
    editor.history.format = "[italic]$text[/italic]".to_string();
    let spans = editor.autosuggestion_spans();
    assert_eq!(
        spans
            .iter()
            .map(|span| span.text.as_str())
            .collect::<String>(),
        " [red]literal[/]"
    );
    assert!(spans[0].style.add_modifier.contains(Modifier::ITALIC));
}

#[test]
fn inline_metadata_follows_the_hint_without_entering_the_command() {
    let mut editor = editor("echo caf");
    editor.rprompt.clear();
    editor.hide_overlay();
    let frame = Scene::for_editor(
        &editor,
        TerminalSize::new(40, 12),
        std::time::Instant::now(),
    )
    .render();
    let line = (0..40)
        .map(|column| frame.buffer[(column, 0)].symbol())
        .collect::<String>();
    assert!(line.starts_with("$ echo café"), "{line}");
    assert_eq!(frame.buffer[(12, 0)].symbol(), "世");
    assert_eq!(frame.buffer[(14, 0)].symbol(), "界");
    assert_eq!(line["$ echo café 世 界   ".len()..].trim_end(), "History");
    assert!(frame.buffer[(18, 0)].modifier.contains(Modifier::ITALIC));
    assert_eq!(frame.required_height, 1);
    editor.handle_key(Action::Right);
    assert_eq!(editor.buffer(), "echo café 世界");
    assert!(editor.autosuggestion_metadata().is_empty());
}

#[test]
fn inline_metadata_disappears_before_overlapping_the_right_prompt() {
    let mut editor = editor("echo caf");
    editor.hide_overlay();
    let frame = Scene::for_editor(
        &editor,
        TerminalSize::new(25, 12),
        std::time::Instant::now(),
    )
    .render();
    let line = (0..25)
        .map(|column| frame.buffer[(column, 0)].symbol())
        .collect::<String>();
    assert!(line.ends_with("RIGHT"), "{line}");
    assert!(!line.contains("History"), "{line}");
}

#[path = "history_navigation_tests.rs"]
mod navigation;

#[test]
fn metadata_style_is_configurable_independently_of_the_suggestion() {
    let mut editor = editor("echo");
    editor.history.metadata_format = "[bold]$text[/bold]".to_string();
    let spans = editor.autosuggestion_metadata();
    assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
    assert!(!spans[0].style.add_modifier.contains(Modifier::ITALIC));
    assert!(
        editor.autosuggestion_spans()[0]
            .style
            .add_modifier
            .contains(Modifier::DIM)
    );
}
