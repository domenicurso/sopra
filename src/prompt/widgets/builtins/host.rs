use super::Widget;

struct Host(String);

impl Widget for Host {
    fn token(&self) -> &str { "{host}" }
    fn render(&self) -> String { self.0.clone() }
}

pub(super) fn create(_context: &super::WidgetContext<'_>) -> Box<dyn Widget> {
    Box::new(Host(sopra_shell_context::var("SOPRA_WIDGET_HOST").unwrap_or_default()
        .chars().filter(|character| !character.is_control()).collect()))
}
