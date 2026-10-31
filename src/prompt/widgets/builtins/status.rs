use sopra::{Context, Text, Widget};

struct Status;
impl Widget for Status {
    fn render(&mut self, context: &Context) -> Text {
        context
            .environment
            .get("SOPRA_WIDGET_STATUS")
            .and_then(|value| value.parse::<i32>().ok())
            .filter(|value| *value != 0)
            .map(|value| value.to_string())
            .unwrap_or_default()
            .into()
    }
}
pub(crate) fn create() -> Box<dyn Widget + Send> {
    Box::new(Status)
}
