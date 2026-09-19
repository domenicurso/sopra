mod names;

use crate::graph::{OptionSpec, ValueAttachment, ValueSpec};

pub(crate) struct ParsedOption {
    pub(crate) name: String,
    pub(crate) value: Option<ValueSpec>,
    pub(crate) attachment: ValueAttachment,
}

pub(crate) fn option_row(left: &str, description: &str) -> Option<OptionSpec> {
    let mut names = Vec::new();
    let mut value = None;
    let mut attachment = ValueAttachment::Separate;
    let tokens = left.split_whitespace().collect::<Vec<_>>();
    for (index, token) in tokens.iter().enumerate() {
        let token = token.trim_matches(',');
        let Some(parsed) = option_token(token) else {
            continue;
        };
        let separate_value = parsed.value.is_none()
            && tokens
                .get(index + 1)
                .is_some_and(|next| !next.starts_with('-'));
        for name in [parsed.name.clone()] {
            if !names.contains(&name) {
                names.push(name);
            }
        }
        value = value.or(parsed.value);
        attachment = attachment.merge(parsed.attachment);
        if value.is_none() && separate_value {
            value = Some(value_spec(
                tokens[index + 1],
                tokens[index + 1].starts_with('['),
            ));
            attachment = ValueAttachment::Separate;
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
        attachment,
        repeatable: description.to_ascii_lowercase().contains("multiple times"),
    })
}

pub(crate) fn option_token(token: &str) -> Option<ParsedOption> {
    let token = token.trim_matches([',', '.']);
    if !token.starts_with('-') || token == "--" {
        return None;
    }
    if let Some(start) = token.find('[').or_else(|| token.find('<')) {
        let name = token[..start].trim_end_matches('=').trim();
        if !names::valid(name) {
            return None;
        }
        let close = if token.as_bytes().get(start) == Some(&b'[') {
            ']'
        } else {
            '>'
        };
        let value = token[start + 1..].trim_end_matches(close);
        return (!name.is_empty() && !value.is_empty()).then(|| {
            let attachment = if value.starts_with('=') || name.starts_with("--") {
                ValueAttachment::Attached
            } else {
                ValueAttachment::Either
            };
            ParsedOption {
                name: name.to_string(),
                value: Some(value_spec(value.trim_start_matches('='), true)),
                attachment,
            }
        });
    }
    if let Some((name, value)) = token.split_once('=') {
        if !names::valid(name) {
            return None;
        }
        let value = if simple_choice(value) {
            literal_value_spec(value)
        } else {
            value_spec(value, false)
        };
        return Some(ParsedOption {
            name: name.to_string(),
            value: Some(value),
            attachment: ValueAttachment::Attached,
        });
    }
    if !names::valid(token) {
        return None;
    }
    Some(ParsedOption {
        name: token.to_string(),
        value: None,
        attachment: ValueAttachment::Separate,
    })
}

pub(crate) fn value_spec(name: &str, optional: bool) -> ValueSpec {
    let mut value = ValueSpec::named(clean_value_name(name));
    value.optional = optional;
    value.choices = choices(name);
    value
}

fn literal_value_spec(value: &str) -> ValueSpec {
    let mut spec = value_spec("value", false);
    spec.literal = Some(value.to_string());
    spec
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

pub(crate) fn clean_description(description: &str) -> Option<String> {
    let value = strip_metadata(description)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    (!value.is_empty()).then_some(value)
}

pub(crate) fn choices_from_description(description: &str) -> Vec<String> {
    let explicit = choices(description);
    if !explicit.is_empty() {
        return explicit;
    }
    let values = parse_choices(description)
        .into_iter()
        .filter(|value| simple_choice(value))
        .collect::<Vec<_>>();
    if values.len() > 1 { values } else { Vec::new() }
}

fn strip_metadata(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find('[') {
        output.push_str(&rest[..start]);
        let Some(end) = metadata_end(rest, start) else {
            output.push_str(&rest[start..]);
            break;
        };
        let tag = &rest[start + 1..end - 1];
        if !metadata_tag(tag) {
            output.push_str(&rest[start..end]);
        }
        rest = &rest[end..];
    }
    if !rest.is_empty() {
        output.push_str(rest);
    }
    output
}

fn metadata_end(value: &str, start: usize) -> Option<usize> {
    let mut depth = 0;
    for (offset, character) in value[start..].char_indices() {
        match character {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(start + offset + 1);
                }
            }
            _ => {}
        }
    }
    None
}

fn metadata_tag(tag: &str) -> bool {
    let label = tag
        .split_once(':')
        .map_or(tag, |(label, _)| label)
        .trim()
        .to_ascii_lowercase();
    matches!(
        label.as_str(),
        "aliases"
            | "array"
            | "boolean"
            | "choices"
            | "default"
            | "number"
            | "possible values"
            | "required"
            | "string"
    )
}

fn description_value(description: &str) -> Option<ValueSpec> {
    let lower = description.to_ascii_lowercase();
    let name = [
        "[string]",
        "[number]",
        "[array]",
        "[file]",
        "[path]",
        "[directory]",
    ]
    .iter()
    .find(|tag| lower.contains(**tag))
    .map(|tag| tag.trim_matches(['[', ']']))
    .or_else(|| lower.contains("requires an argument").then_some("value"))?;
    let mut value = value_spec(name, false);
    value.choices = choices(description);
    Some(value)
}

fn choices(text: &str) -> Vec<String> {
    let lower = text.to_ascii_lowercase();
    for label in ["[choices:", "[possible values:"] {
        let Some(start) = lower.find(label) else {
            continue;
        };
        let content_start = start + label.len();
        let content_end = text[content_start..]
            .find(']')
            .map_or(text.len(), |offset| content_start + offset);
        let values = parse_choices(&text[content_start..content_end]);
        if !values.is_empty() {
            return values;
        }
    }
    for marker in [
        "valid options are",
        "supported values are",
        "supported formats are",
        "supported values:",
        "supported formats:",
        "possible values are",
        "options are",
        "options:",
        "choices are",
    ] {
        if let Some(start) = lower.find(marker) {
            let values = prose_choices(&text[start + marker.len()..]);
            if values.len() > 1 {
                return values;
            }
        }
    }
    let Some((open, close)) = [('{', '}'), ('(', ')')]
        .into_iter()
        .find(|(open, close)| text.contains(*open) && text.contains(*close))
    else {
        return Vec::new();
    };
    let Some(start) = text.find(open) else {
        return Vec::new();
    };
    let Some(end) = text[start + 1..].find(close) else {
        return Vec::new();
    };
    let values = parse_choices(&text[start + 1..start + 1 + end]);
    if values.len() > 1 && values.iter().all(|value| simple_choice(value)) {
        values
    } else {
        Vec::new()
    }
}

fn prose_choices(text: &str) -> Vec<String> {
    let mut values = Vec::new();
    for part in text.split(',') {
        for choice in part.split(" and ") {
            let choice = choice
                .split_once('(')
                .map_or(choice, |(value, _)| value)
                .trim()
                .trim_matches(['\'', '"']);
            let Some(value) = choice.split_whitespace().next() else {
                continue;
            };
            if simple_choice(value) && !values.contains(&value.to_string()) {
                values.push(value.to_string());
            }
        }
    }
    values
}

fn parse_choices(text: &str) -> Vec<String> {
    text.split(['|', ','])
        .map(|value| value.trim().trim_matches(['\'', '"']))
        .map(|value| value.strip_prefix("or ").unwrap_or(value))
        .map(|value| value.strip_prefix("and ").unwrap_or(value))
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn simple_choice(value: &str) -> bool {
    value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "._+:/-".contains(character))
}

fn clean_value_name(name: &str) -> String {
    name.trim_matches(['<', '>', '[', ']', '='])
        .trim()
        .to_string()
}
