use super::{CommandGraph, CommandNode};
impl CommandGraph {
    pub(crate) fn useful(&self) -> bool {
        self.root.useful()
    }

    pub(crate) fn merge(&mut self, incoming: Self) {
        self.root.merge(incoming.root);
    }

    pub(crate) fn merge_at(&mut self, path: &[String], incoming: CommandNode) -> bool {
        self.root.merge_at(path, incoming)
    }

    pub(crate) fn sort(&mut self) {
        self.root.sort();
    }
}

impl CommandNode {
    pub(super) fn sort(&mut self) {
        self.options.sort_by(|left, right| {
            left.names
                .first()
                .cmp(&right.names.first())
                .then_with(|| left.names.len().cmp(&right.names.len()))
        });
        self.subcommands.sort_by(|left, right| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
                .then_with(|| left.name.cmp(&right.name))
        });
        for child in &mut self.subcommands {
            child.sort();
        }
    }
}
