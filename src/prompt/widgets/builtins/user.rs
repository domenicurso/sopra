use sopra::{Context, Text, Widget};

struct User;
impl Widget for User {
    fn render(&mut self, context: &Context) -> Text {
        context
            .environment
            .get("SOPRA_WIDGET_USER")
            .map(|value| {
                value
                    .chars()
                    .filter(|ch| !ch.is_control())
                    .collect::<String>()
            })
            .unwrap_or_default()
            .into()
    }
}
pub(crate) fn create() -> Box<dyn Widget + Send> {
    Box::new(User)
}
