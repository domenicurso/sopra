use super::{Path, PathStyles, Widget};
use crate::{input::Key, prompt::parse};
use ratatui::style::{Color, Modifier};
use std::path::PathBuf;

#[test]
fn focus_styles_segments_without_changing_path_text() {
    let mut path = Path::new(PathBuf::from("/tmp/alpha/beta"));
    path.styles = PathStyles {
        before: "[green]{text}[/]".into(),
        selected: "[yellow][reverse]{text}[/][/]".into(),
        after: "[red][dim]{text}[/][/]".into(),
    };
    let before = path.render();
    path.handle(Key::PromptWidgets);
    let selected = parse(&path.render());
    assert_eq!(
        selected
            .iter()
            .map(|span| span.text.as_str())
            .collect::<String>(),
        before
    );
    assert_eq!(selected.last().map(|span| span.text.as_str()), Some("beta"));
    assert_eq!(
        selected.last().and_then(|span| span.style.fg),
        Some(Color::Yellow)
    );
    assert!(
        selected
            .last()
            .is_some_and(|span| span.style.add_modifier.contains(Modifier::REVERSED))
    );
    path.handle(Key::Tab);
    let parent = parse(&path.render());
    assert_eq!(
        parent
            .iter()
            .map(|span| span.text.as_str())
            .collect::<String>(),
        before
    );
    assert_segment_styles(&parent);
    path.handle(Key::BackTab);
    assert_eq!(parse(&path.render()), selected);
}

fn assert_segment_styles(spans: &[crate::prompt::PromptSpan]) {
    assert!(
        spans.iter().any(
            |span| span.text == "alpha" && span.style.add_modifier.contains(Modifier::REVERSED)
        )
    );
    assert!(spans.iter().any(|span| span.text == "/beta"
        && span.style.fg == Some(Color::Red)
        && span.style.add_modifier.contains(Modifier::DIM)));
    assert!(
        spans
            .iter()
            .any(|span| span.text == "/tmp/" && span.style.fg == Some(Color::Green))
    );
}
