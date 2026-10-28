use super::text_selection::TextSelection;
use crate::input::MouseKind;
use std::time::{Duration, Instant};

#[test]
fn clicks_and_drags_select_graphemes_words_and_lines() {
    let mut selection = TextSelection::default();
    let now = Instant::now();
    let text = "one café 👨‍👩‍👧 end";
    selection.mouse(MouseKind::Press, 4, text, now);
    selection.mouse(MouseKind::Release, 4, text, now);
    selection.mouse(MouseKind::Press, 4, text, now + Duration::from_millis(100));
    assert_eq!(selection.range().map(|range| &text[range]), Some("café"));
    selection.mouse(MouseKind::Drag, text.len(), text, now);
    assert_eq!(
        selection.range().map(|range| &text[range]),
        Some("café 👨‍👩‍👧 end")
    );
    selection.mouse(MouseKind::Release, text.len(), text, now);
    selection.mouse(MouseKind::Press, 4, text, now + Duration::from_millis(200));
    assert_eq!(selection.range(), Some(0..text.len()));
}

#[test]
fn single_click_drag_works_in_both_directions() {
    let mut selection = TextSelection::default();
    let now = Instant::now();
    selection.mouse(MouseKind::Press, 5, "hello world", now);
    selection.mouse(MouseKind::Drag, 1, "hello world", now);
    assert_eq!(selection.range(), Some(1..5));
    selection.mouse(MouseKind::Drag, 8, "hello world", now);
    assert_eq!(selection.range(), Some(5..8));
}

#[test]
fn selected_text_is_rendered_and_replaced_after_resize() {
    use crate::{
        input::{CursorPosition, Key, MouseEvent, TerminalSize},
        scene::Scene,
    };
    use ratatui::style::Modifier;
    let mut editor = test_editor();
    let position = CursorPosition { row: 3, column: 8 };
    for kind in [MouseKind::Press, MouseKind::Release, MouseKind::Press] {
        editor.handle_mouse(MouseEvent { kind, position }, Some(3));
    }
    assert_eq!(editor.text_selection(), Some(6..11));
    let frame = Scene::for_editor(&editor, TerminalSize::new(40, 20), Instant::now()).render();
    assert!(frame.buffer[(8, 0)].modifier.contains(Modifier::REVERSED));
    editor.handle_key(Key::Character('X'));
    assert_eq!(editor.buffer(), "hello X");
    assert_eq!(editor.text_selection(), None);
}

fn test_editor() -> super::EditorState {
    use crate::{
        input::{CursorPosition, TerminalSize},
        palette::TerminalPalette,
    };
    super::EditorState::new(super::EditorConfig {
        buffer: "hello world".into(),
        cursor_chars: 11,
        arm_completion: false,
        suppress_completion: true,
        completion_enabled: false,
        syntax_highlighting: false,
        cursor_animation: false,
        prompt: "$ ".into(),
        rprompt: String::new(),
        anchor: CursorPosition { row: 0, column: 0 },
        size: TerminalSize::new(80, 20),
        cwd: std::path::PathBuf::new(),
        palette: TerminalPalette::default(),
    })
}

#[test]
fn selected_text_deletes_or_collapses_with_arrow_keys() {
    use crate::input::Key;
    let mut editor = test_editor();
    let now = Instant::now();
    editor
        .text_selection
        .mouse(MouseKind::Press, 6, "hello world", now);
    editor
        .text_selection
        .mouse(MouseKind::Drag, 11, "hello world", now);
    editor.handle_key(Key::Left);
    assert_eq!(editor.cursor, 6);
    assert_eq!(editor.buffer(), "hello world");
    editor
        .text_selection
        .mouse(MouseKind::Press, 6, "hello world", now);
    editor
        .text_selection
        .mouse(MouseKind::Drag, 11, "hello world", now);
    editor.handle_key(Key::Backspace);
    assert_eq!(editor.buffer(), "hello ");
    assert_eq!(editor.cursor, 6);
}
