mod choices;
mod metadata;
mod names;
mod tokens;
mod values;
mod words;
use choices::choices;
pub(crate) use choices::choices_from_description;
pub(crate) use metadata::clean_description;
pub(crate) use tokens::option_token;
use values::description_value;
pub(crate) use values::value_spec;
pub(crate) use words::words;

use crate::graph::{OptionSpec, ValueAttachment, ValueSpec};

pub(crate) struct ParsedOption {
    pub(crate) name: String,
    pub(crate) value: Option<ValueSpec>,
    pub(crate) attachment: ValueAttachment,
}

pub(crate) fn option_row(left: &str, description: &str) -> Option<OptionSpec> {
    let mut names = Vec::new();
    let mut value = None;
    let mut attachment: Option<ValueAttachment> = None;
    let tokens = words(left);
    for (index, token) in tokens.iter().enumerate() {
        let token = token.trim_matches(',');
        let Some(parsed) = option_token(token) else {
            continue;
        };
        let separate_value = parsed.value.is_none()
            && tokens
                .get(index + 1)
                .is_some_and(|next| !next.starts_with('-'));
        if !names.contains(&parsed.name) {
            names.push(parsed.name);
        }
        value = value.or(parsed.value);
        attachment = Some(attachment.map_or(parsed.attachment, |current| {
            current.merge(parsed.attachment)
        }));
        if value.is_none() && separate_value {
            value = Some(value_spec(
                &tokens[index + 1],
                tokens[index + 1].starts_with('['),
            ));
            attachment = Some(ValueAttachment::Separate);
        }
    }
    if names.is_empty() {
        return None;
    }
    if value.is_none() {
        value = description_value(description);
    }
    if let Some(value) = &mut value {
        for choice in choices(description) {
            if !value.choices.contains(&choice) {
                value.choices.push(choice);
            }
        }
    }
    Some(OptionSpec {
        names,
        description: clean_description(description),
        value,
        attachment: attachment.unwrap_or_default(),
        repeatable: metadata::repeatable(description),
    })
}

pub(crate) fn is_option_token(token: &str) -> bool {
    token.starts_with('-') && token != "--"
}

pub(crate) fn is_short_cluster(name: &str) -> bool {
    let Some(short) = name.strip_prefix('-') else {
        return false;
    };
    let count = short.chars().count();
    count > 2
        && (count <= 4
            || short
                .chars()
                .any(|character| character.is_ascii_uppercase() || character.is_ascii_digit()))
}

pub(crate) fn placeholder(token: &str) -> bool {
    token.starts_with(['<', '[']) || token.ends_with("]")
}

pub(crate) fn synthetic_placeholder(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "args" | "arguments" | "command" | "flags" | "option" | "options"
    )
}
