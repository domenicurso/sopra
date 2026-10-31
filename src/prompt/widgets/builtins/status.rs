use super::Widget;

struct Status(String);

impl Widget for Status {
    fn token(&self) -> &str { "{status}" }
    fn render(&self) -> String { self.0.clone() }
}

pub(super) fn create(_context: &super::WidgetContext<'_>) -> Box<dyn Widget> {
    Box::new(Status(sopra_shell_context::var("SOPRA_WIDGET_STATUS").ok()
        .and_then(|value| value.parse::<i32>().ok()).filter(|value| *value != 0)
        .map(|value| value.to_string()).unwrap_or_default()))
}
