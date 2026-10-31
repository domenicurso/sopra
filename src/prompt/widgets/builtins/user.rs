use super::Widget;

struct User(String);

impl Widget for User {
    fn token(&self) -> &str { "{user}" }
    fn render(&self) -> String { self.0.clone() }
}

pub(super) fn create(_context: &super::WidgetContext<'_>) -> Box<dyn Widget> {
    Box::new(User(sopra_shell_context::var("SOPRA_WIDGET_USER").unwrap_or_default()
        .chars().filter(|character| !character.is_control()).collect()))
}
