use super::model::Model;
pub(super) struct Layout {
    pub(super) height: usize,
    pub(super) prompt_top: u16,
    pub(super) prompt_source: u16,
    pub(super) pinned: u16,
}
impl Layout {
    pub(super) fn new(model: &Model) -> Self {
        let screen = model.terminal.screen();
        let source = model
            .prompt
            .filter(|_| !model.executing && !screen.alternate_screen());
        let last = (0..model.size.rows)
            .rev()
            .find(|row| {
                (0..model.size.cols).any(|column| {
                    screen
                        .cell(*row, column)
                        .is_some_and(|cell| cell.has_contents())
                })
            })
            .unwrap_or(0);
        let pinned = source.map_or(0, |source| {
            last.saturating_sub(source)
                .saturating_add(1)
                .min(model.size.rows / 2)
                .max(1)
        });
        let prompt_top = model.size.rows.saturating_sub(pinned);
        Self {
            height: usize::from(prompt_top.saturating_sub(2)),
            prompt_top,
            prompt_source: source.map_or(0, |source| {
                source.max(
                    screen
                        .cursor_position()
                        .0
                        .saturating_add(1)
                        .saturating_sub(pinned),
                )
            }),
            pinned,
        }
    }
}
