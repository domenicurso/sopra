use sopra::{Context, Text, Widget};

struct GitStatus;
impl Widget for GitStatus {
    fn render(&mut self, context: &Context) -> Text {
        super::git::snapshot(context).map_or_else(Text::new, |snapshot| snapshot.status(context))
    }
}
pub(crate) fn create() -> Box<dyn Widget + Send> {
    Box::new(GitStatus)
}
