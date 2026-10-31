use super::Widget;

struct Shell(String);

impl Widget for Shell {
    fn token(&self) -> &str { "{shell}" }
    fn render(&self) -> String { self.0.clone() }
}

pub(super) fn create(_context: &super::WidgetContext<'_>) -> Box<dyn Widget> {
    Box::new(Shell("zsh".to_owned()))
}
