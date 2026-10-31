use super::super::{Key, Widget};
use super::*;

#[test]
fn messages_preserve_emphasis_context_and_typed_events() -> io::Result<()> {
    let context = Context {
        cwd: "/one/two".into(),
        options: BTreeMap::from([("label".into(), "a\nb".into())]),
        ..Context::default()
    };
    for event in [
        None,
        Some(Event::Activate),
        Some(Event::Key(Key::Tab)),
        Some(Event::Key(Key::Character('🦀'))),
    ] {
        assert_eq!(
            read_request(&request(&context, event))?,
            (context.clone(), event)
        );
    }
    let value = Response {
        text: Text::new()
            .part(super::super::Emphasis::Dim, "[")
            .push("3!")
            .part(super::super::Emphasis::Dim, "]"),
        action: Action::ChangeDirectory("/one".into()),
        active: true,
        interactive: true,
        refresh: Refresh::Every(Duration::from_secs(1)),
    };
    assert_eq!(read_response(&response(&value))?, value);
    assert_eq!(
        read_response(&response(&Response::default()))?,
        Response::default()
    );
    Ok(())
}

#[test]
fn malformed_messages_and_terminal_controls_are_rejected() {
    for line in ["W1\t\t\t\t", "W2\t\t\t\tunknown", "W2\t\t\t\tchar:110000"] {
        assert!(read_request(line).is_err());
    }
    for line in [
        "W2\t0\t0\t-\tignore\t:1b",
        "W2\t0\t0\t-\tignore\t:ff",
        "W2\t2\t0\t-\tignore\t",
        "W2\t0\t0\t0\tignore\t",
    ] {
        assert!(read_response(line).is_err());
    }
}

#[test]
fn ordinary_widget_uses_the_same_contract_as_remote_widgets() {
    struct Demo;
    impl Widget for Demo {
        fn render(&mut self, context: &Context) -> Text {
            context.option("label", "demo").into()
        }
    }
    let mut widget: Box<dyn Widget> = Box::new(Demo);
    assert_eq!(widget.render(&Context::default()), Text::from("demo"));
    assert_eq!(
        widget.handle(Event::Activate, &Context::default()),
        Action::Ignored
    );
}
