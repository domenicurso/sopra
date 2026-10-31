use sopra::{Context, Text, Widget};

struct Shell;
impl Widget for Shell {
    fn render(&mut self, _context: &Context) -> Text {
        "zsh".into()
    }
}
pub(crate) fn create() -> Box<dyn Widget + Send> {
    Box::new(Shell)
}
