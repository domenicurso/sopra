use super::Widgets;
use crate::actions::Action;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

fn wait(widgets: &mut Widgets, expected: &str) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        let text = widgets
            .render()
            .0
            .iter()
            .map(|s| s.text.as_str())
            .collect::<String>();
        if text.contains(expected) {
            return;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("widget did not render {expected:?}");
}

#[test]
fn path_selection_keeps_its_text_and_delivers_directory_actions_asynchronously() {
    let mut widgets = Widgets::new("$path $ ".into(), String::new(), PathBuf::from("/one/two"));
    wait(&mut widgets, "/one/two");
    assert_eq!(
        widgets.handle(Action::PromptWidgets),
        sopra::Action::Consumed
    );
    assert!(widgets.active());
    widgets.handle(Action::Tab);
    wait(&mut widgets, "/one/two");
    widgets.handle(Action::Enter);
    assert!(widgets.active());
    assert_eq!(widgets.handle(Action::Enter), sopra::Action::Consumed);
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(action) = widgets.poll_action() {
            assert_eq!(action, sopra::Action::ChangeDirectory("/one".into()));
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(!widgets.active());
}

#[test]
fn widget_values_are_never_reinterpreted_as_templates() {
    let mut widgets = Widgets::new("$path".into(), String::new(), "/one/$clock".into());
    wait(&mut widgets, "/one/$clock");
    assert!(!widgets.widgets.iter().any(|entry| entry.name == "clock"));
}

#[test]
fn unknown_references_and_malformed_templates_report_configuration_errors() {
    for source in ["$unknown", "[red]$path[/blue]"] {
        assert!(
            Widgets::configured(
                source.into(),
                String::new(),
                PathBuf::from("."),
                &super::settings::Settings::default()
            )
            .is_err()
        );
    }
}
