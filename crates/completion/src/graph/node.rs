use super::Synopsis;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CommandGraph {
    pub(crate) root: CommandNode,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CommandNode {
    pub(crate) name: String,
    pub(crate) aliases: Vec<String>,
    pub(crate) description: Option<String>,
    pub(crate) options: Vec<OptionSpec>,
    pub(crate) positionals: Vec<PositionalSpec>,
    pub(crate) subcommands: Vec<CommandNode>,
    pub(crate) synopsis: Option<Synopsis>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OptionSpec {
    pub(crate) names: Vec<String>,
    pub(crate) description: Option<String>,
    pub(crate) value: Option<ValueSpec>,
    pub(crate) attachment: ValueAttachment,
    pub(crate) repeatable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ValueAttachment {
    #[default]
    Separate,
    Attached,
    Either,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PositionalSpec {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) value: ValueSpec,
    pub(crate) optional: bool,
    pub(crate) repeatable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValueSpec {
    pub(crate) name: String,
    pub(crate) kind: ValueKind,
    pub(crate) choices: Vec<String>,
    pub(crate) optional: bool,
    pub(crate) literal: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ValueKind {
    #[default]
    Text,
    Number,
    File,
    Directory,
    Url,
}

impl CommandNode {
    pub(crate) fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    pub(crate) fn useful(&self) -> bool {
        !self.options.is_empty()
            || !self.positionals.is_empty()
            || !self.subcommands.is_empty()
            || self.description.is_some()
    }

    pub(crate) fn merge(&mut self, incoming: Self) {
        self.description = merge_description(self.description.take(), incoming.description);
        self.synopsis = self.synopsis.take().or(incoming.synopsis);
        for option in incoming.options {
            self.merge_option(option);
        }
        for (index, positional) in incoming.positionals.into_iter().enumerate() {
            self.merge_positional_at(index, positional);
        }
        for subcommand in incoming.subcommands {
            self.merge_subcommand(subcommand);
        }
    }

    pub(crate) fn merge_option(&mut self, incoming: OptionSpec) {
        let Some(existing) = self.options.iter_mut().find(|option| {
            option
                .names
                .iter()
                .any(|name| incoming.names.iter().any(|other| other == name))
        }) else {
            self.options.push(incoming);
            return;
        };
        merge_names(&mut existing.names, incoming.names);
        existing.description = merge_description(existing.description.take(), incoming.description);
        existing.value = match (existing.value.take(), incoming.value) {
            (Some(current), Some(incoming)) => Some(current.merge(incoming)),
            (current, incoming) => current.or(incoming),
        };
        existing.attachment = existing.attachment.merge(incoming.attachment);
        existing.repeatable |= incoming.repeatable;
    }

    pub(crate) fn merge_positional(&mut self, incoming: PositionalSpec) {
        let Some(index) = self
            .positionals
            .iter()
            .position(|positional| positional.name == incoming.name)
        else {
            self.positionals.push(incoming);
            return;
        };
        self.merge_positional_at(index, incoming);
    }

    pub(crate) fn merge_positional_at(&mut self, index: usize, incoming: PositionalSpec) {
        let Some(existing) = self.positionals.get_mut(index) else {
            self.positionals.push(incoming);
            return;
        };
        existing.description = merge_description(existing.description.take(), incoming.description);
        existing.value = existing.value.clone().merge(incoming.value);
        existing.optional &= incoming.optional;
        existing.repeatable |= incoming.repeatable;
    }

    pub(crate) fn merge_subcommand(&mut self, incoming: CommandNode) {
        let Some(existing) = self.subcommands.iter_mut().find(|command| {
            command.name == incoming.name
                || command
                    .aliases
                    .iter()
                    .any(|alias| incoming.aliases.contains(alias))
        }) else {
            self.subcommands.push(incoming);
            return;
        };
        existing.aliases = merge_vec(&existing.aliases, &incoming.aliases);
        existing.merge(incoming);
    }

    pub(crate) fn merge_at(&mut self, path: &[String], incoming: CommandNode) -> bool {
        if path.is_empty() {
            self.merge(incoming);
            return true;
        }
        let Some(child) = self.subcommands.iter_mut().find(|command| {
            command.name == path[0] || command.aliases.iter().any(|alias| alias == &path[0])
        }) else {
            return false;
        };
        let mut incoming = incoming;
        if path.len() == 1 && incoming.description == self.description {
            incoming.description = None;
        }
        child.merge_at(&path[1..], incoming)
    }

    pub(crate) fn find(&self, path: &[String]) -> Option<&Self> {
        path.iter().try_fold(self, |command, name| {
            command.subcommands.iter().find(|child| {
                child.name == *name || child.aliases.iter().any(|alias| alias == name)
            })
        })
    }
}

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
    fn sort(&mut self) {
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

fn merge_names(target: &mut Vec<String>, values: Vec<String>) {
    for value in values {
        if !target.contains(&value) {
            target.push(value);
        }
    }
}

fn merge_description(current: Option<String>, incoming: Option<String>) -> Option<String> {
    match (current, incoming) {
        (Some(current), Some(incoming)) => Some(
            if description_quality(&incoming) > description_quality(&current) {
                incoming
            } else {
                current
            },
        ),
        (current, incoming) => current.or(incoming),
    }
}

fn description_quality(value: &str) -> (bool, usize) {
    let value = value.trim();
    let useful = !value.is_empty() && !matches!(value, "-" | "—");
    (useful, value.chars().count())
}

impl ValueAttachment {
    pub(crate) fn merge(self, incoming: Self) -> Self {
        if self == incoming {
            return self;
        }
        Self::Either
    }
}

fn merge_vec<T: Clone + PartialEq>(left: &[T], right: &[T]) -> Vec<T> {
    let mut merged = left.to_vec();
    for value in right {
        if !merged.contains(value) {
            merged.push(value.clone());
        }
    }
    merged
}
