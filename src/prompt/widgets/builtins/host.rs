use sopra::{Context, Text, Widget};

struct Host;
impl Widget for Host {
    fn render(&mut self, context: &Context) -> Text {
        context
            .environment
            .get("SOPRA_WIDGET_HOST")
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
    Box::new(Host)
}
