use super::History;
use crate::{
    editor::{EditorConfig, EditorState},
    input::{CursorPosition, Key, TerminalSize},
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
        entries: vec!["echo café 世界".to_string(), "echo older".to_string()],
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
    editor.handle_key(Key::Escape);
    assert!(editor.autosuggestion_spans().is_empty());
    editor.handle_key(Key::Character('é'));
    assert!(!editor.autosuggestion_spans().is_empty());
    editor.handle_key(Key::Left);
    assert!(editor.autosuggestion_spans().is_empty());
}

#[test]
fn right_end_and_word_right_accept_history_without_splitting_unicode() {
    for key in [Key::Right, Key::End] {
        let mut editor = editor("echo caf");
        editor.handle_key(key);
        assert_eq!(editor.buffer(), "echo café 世界");
        assert_eq!(editor.cursor_byte(), editor.buffer().len());
    }
    let mut editor = editor("echo");
    editor.handle_key(Key::WordRight);
    assert_eq!(editor.buffer(), "echo café");
    editor.handle_key(Key::WordRight);
    assert_eq!(editor.buffer(), "echo café 世界");
}

#[test]
fn history_menu_replaces_the_entire_command_without_adding_a_space() {
    let mut editor = editor("echo");
    assert_eq!(editor.visible_items()[0].insert, "echo café 世界");
    editor.handle_key(Key::Tab);
    assert!(editor.autosuggestion_spans().is_empty());
    editor.handle_key(Key::Tab);
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
fn history_snapshot_preserves_multiline_entries_and_deduplicates_recent_commands()
-> std::io::Result<()> {
    let file = std::env::temp_dir().join(format!("sopra-history-{}", std::process::id()));
    std::fs::write(
        &file,
        b"echo recent\0echo recent\0echo old\necho next\0\xff\0\x1b[31m\0",
    )?;
    let history = History::read(Some(&file))?;
    std::fs::remove_file(file)?;
    assert_eq!(history.entries, ["echo recent", "echo old\necho next"]);
    Ok(())
}

#[test]
fn disabling_inline_hints_keeps_history_completion_available() {
    let mut editor = editor("echo");
    editor.history.enabled = false;
    assert!(editor.autosuggestion_spans().is_empty());
    assert!(!editor.visible_items().is_empty());
    editor.handle_key(Key::Right);
    assert_eq!(editor.buffer(), "echo");
}

#[test]
fn configured_styling_does_not_interpret_history_as_markup() {
    let mut editor = editor("echo");
    editor.history.entries = vec!["echo [red]literal[/]".to_string()];
    editor.history.format = "[italic]{text}[/]".to_string();
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
