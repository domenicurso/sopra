use super::{ValueKind, ValueSpec};

impl ValueSpec {
    pub(crate) fn named(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            kind: infer_kind(&name),
            name,
            choices: Vec::new(),
            optional: false,
            literal: None,
        }
    }

    pub(crate) fn choice(name: impl Into<String>) -> Self {
        let name = name.into();
        let mut value = Self::named(&name);
        value.choices.push(name);
        value
    }

    pub(super) fn merge(mut self, incoming: Self) -> Self {
        let names_match = self.name.is_empty() || self.name == incoming.name;
        if self.name.is_empty() {
            self.name = incoming.name;
        }
        if !names_match {
            self.kind = ValueKind::Text;
        } else if self.kind == ValueKind::Text {
            self.kind = incoming.kind;
        }
        if let (Some(current), Some(incoming)) = (&self.literal, &incoming.literal)
            && current != incoming
        {
            self.choices = merge_choices(&self.choices, &[current.clone(), incoming.clone()]);
        }
        self.choices = merge_choices(&self.choices, &incoming.choices);
        self.optional &= incoming.optional;
        self.literal = self.literal.or(incoming.literal);
        self
    }
}

fn infer_kind(name: &str) -> ValueKind {
    let name = name.to_ascii_lowercase();
    if name.contains("dir") || name.contains("folder") {
        ValueKind::Directory
    } else if name.contains("file") || name.contains("path") {
        ValueKind::File
    } else if name.contains("url") || name.contains("uri") || name.contains("link") {
        ValueKind::Url
    } else if name.contains("num") || name.contains("count") || name.contains("port") {
        ValueKind::Number
    } else {
        ValueKind::Text
    }
}

fn merge_choices(left: &[String], right: &[String]) -> Vec<String> {
    let mut merged = left.to_vec();
    for value in right {
        if !merged.contains(value) {
            merged.push(value.clone());
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::{ValueKind, ValueSpec};

    #[test]
    fn alternatives_with_different_names_do_not_force_file_completion() {
        let value = ValueSpec::named("path").merge(ValueSpec::named("commit"));

        assert_eq!(value.kind, ValueKind::Text);
    }
}
