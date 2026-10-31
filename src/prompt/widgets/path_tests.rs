use super::Path;
use sopra::{Context, Event, Key, Widget};

#[test]
fn focus_returns_named_parts_without_changing_path_text() {
    let context = Context {
        cwd: "/tmp/alpha/beta".into(),
        ..Context::default()
    };
    let mut path = Path::default();
    let before = path.render(&context);
    path.handle(Event::Activate, &context);
    let selected = path.render(&context);
    let plain = |text: &sopra::Text| {
        text.parts
            .iter()
            .map(|part| part.text.as_str())
            .collect::<String>()
    };
    assert_eq!(plain(&selected), plain(&before));
    assert_eq!(selected.parts[1].emphasis, sopra::Emphasis::Selected);
    assert_eq!(selected.parts[1].text, "beta");
    path.handle(Event::Key(Key::Tab), &context);
    let parent = path.render(&context);
    assert_eq!(plain(&parent), plain(&before));
    assert_eq!(parent.parts[0].text, "/tmp/");
    assert_eq!(parent.parts[1].text, "alpha");
    assert_eq!(parent.parts[2].text, "/beta");
    path.handle(Event::Key(Key::BackTab), &context);
    assert_eq!(path.render(&context), selected);
}
