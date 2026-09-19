use super::super::option;
use crate::graph::{OptionSpec, Synopsis, ValueAttachment, ValueSpec};

pub(super) fn option_specs(token: &str, next: Option<&Synopsis>) -> Vec<OptionSpec> {
    let Some(parsed) = option::option_token(token) else {
        return Vec::new();
    };
    let name = parsed.name;
    if name.starts_with("--")
        || name.len() <= 2
        || parsed.value.is_some()
        || has_value_placeholder(next)
        || !option::is_short_cluster(&name)
    {
        return vec![OptionSpec {
            names: vec![name],
            description: None,
            value: parsed.value,
            attachment: parsed.attachment,
            repeatable: false,
        }];
    }
    name[1..]
        .chars()
        .map(|character| OptionSpec {
            names: vec![format!("-{character}")],
            description: None,
            value: None,
            attachment: ValueAttachment::Separate,
            repeatable: false,
        })
        .collect()
}

fn has_value_placeholder(node: Option<&Synopsis>) -> bool {
    match node {
        Some(Synopsis::Token(token)) => option::placeholder(token),
        Some(Synopsis::Optional(nodes) | Synopsis::Sequence(nodes)) => nodes
            .first()
            .is_some_and(|node| has_value_placeholder(Some(node))),
        _ => false,
    }
}

pub(super) fn next_value(
    node: Option<&Synopsis>,
    options: &[OptionSpec],
) -> (Option<ValueSpec>, usize, ValueAttachment) {
    let name = options
        .first()
        .and_then(|option| option.names.first())
        .map(String::as_str)
        .unwrap_or_default();
    match node {
        Some(Synopsis::Token(token)) if !option::is_option_token(token) && token != "..." => (
            Some(option::value_spec(token, false)),
            1,
            ValueAttachment::Separate,
        ),
        Some(Synopsis::Optional(nodes)) => {
            let Some(Synopsis::Token(token)) = nodes.first() else {
                return (None, 0, ValueAttachment::Separate);
            };
            let token = token.trim_start_matches('=');
            if token.is_empty() {
                (None, 0, ValueAttachment::Separate)
            } else {
                let attachment = if name.starts_with("--")
                    && nodes.first().is_some_and(
                        |node| matches!(node, Synopsis::Token(value) if value.starts_with('=')),
                    ) {
                    ValueAttachment::Attached
                } else {
                    ValueAttachment::Either
                };
                (Some(option::value_spec(token, true)), 1, attachment)
            }
        }
        _ => (None, 0, ValueAttachment::Separate),
    }
}
