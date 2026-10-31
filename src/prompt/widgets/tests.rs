use super::{WidgetAction, Widgets};
use crate::input::Key;
use std::path::PathBuf;

#[test]
fn path_widget_selects_parents_and_can_cancel() {
    let mut widgets = Widgets::new("{path} $ ".into(), String::new(), PathBuf::from("/one/two"));
    assert!(matches!(
        widgets.handle(Key::PromptWidgets),
        WidgetAction::Consumed
    ));
    assert!(
        crate::prompt::parse(&widgets.render().0)
            .iter()
            .map(|span| span.text.as_str())
            .collect::<String>()
            .starts_with("/one/two")
    );
    widgets.handle(Key::Tab);
    assert!(
        crate::prompt::parse(&widgets.render().0)
            .iter()
            .map(|span| span.text.as_str())
            .collect::<String>()
            .starts_with("/one/two")
    );
    assert!(
        matches!(widgets.handle(Key::Enter), WidgetAction::ChangeDirectory(path) if path == std::path::Path::new("/one"))
    );
    widgets.handle(Key::PromptWidgets);
    widgets.handle(Key::Escape);
    assert!(matches!(widgets.handle(Key::Enter), WidgetAction::Ignored));
}

#[test]
fn only_registered_tokens_in_the_template_are_expanded() {
    let mut widgets = Widgets::new(
        "{path} {unknown}".into(),
        String::new(),
        PathBuf::from("/one/{clock}"),
    );
    assert_eq!(widgets.render().0, "/one/{clock} {unknown}");
    widgets = Widgets::new("plain".into(), String::new(), PathBuf::from("/one"));
    assert!(matches!(
        widgets.handle(Key::PromptWidgets),
        WidgetAction::Ignored
    ));
}

struct Interactive(bool);
impl super::Widget for Interactive {
    fn token(&self) -> &str {
        "{interactive}"
    }
    fn render(&self) -> String {
        if self.0 { "focused" } else { "idle" }.into()
    }
    fn interactive(&self) -> bool {
        true
    }
    fn active(&self) -> bool {
        self.0
    }
    fn handle(&mut self, key: Key) -> WidgetAction {
        match key {
            Key::PromptWidgets => self.0 = true,
            Key::Escape => self.0 = false,
            Key::Enter => return WidgetAction::ChangeDirectory(PathBuf::from("/chosen")),
            _ => {}
        }
        WidgetAction::Consumed
    }
}

#[test]
fn activation_cycles_focus_and_routes_keys_to_the_active_widget() {
    let mut widgets = Widgets::new(
        "{path} {interactive}".into(),
        String::new(),
        "/one/two".into(),
    );
    widgets.widgets.push(Box::new(Interactive(false)));
    widgets.handle(Key::PromptWidgets);
    assert!(widgets.widgets[0].active());
    widgets.handle(Key::PromptWidgets);
    assert!(!widgets.widgets[0].active());
    assert!(widgets.render().0.ends_with("focused"));
    assert!(
        matches!(widgets.handle(Key::Enter), WidgetAction::ChangeDirectory(path) if path == std::path::Path::new("/chosen"))
    );
    widgets.handle(Key::PromptWidgets);
    assert!(widgets.widgets[0].active());
    assert!(widgets.render().0.ends_with("idle"));
}
