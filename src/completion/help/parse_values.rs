use super::{
    model::{ArgumentKind, HelpOption, HelpPositional},
    parse_rows::Row,
};

pub(crate) fn parse_options(rows: &[Row]) -> Vec<HelpOption> {
    let parsed = rows
        .iter()
        .filter_map(|row| {
            let names = option_names(&row.left);
            if names.is_empty() {
                return None;
            }
            let values = bracket_value(&row.description, "choices:")
                .or_else(|| bracket_value(&row.description, "possible values:"))
                .map(parse_values)
                .filter(|values| !values.is_empty())
                .or_else(|| inline_values(&row.left))
                .unwrap_or_default();
            Some(HelpOption {
                expects_value: !values.is_empty()
                    || has_value_tag(&row.description)
                    || row.left.contains(['<', '='])
                    || option_has_argument(&row.left),
                names,
                description: clean_description(&row.description),
                values,
            })
        })
        .collect::<Vec<_>>();
    let mut merged = Vec::new();
    for option in parsed {
        let Some(existing) = merged.iter_mut().find(|existing: &&mut HelpOption| {
            existing
                .names
                .iter()
                .any(|name| option.names.contains(name))
        }) else {
            merged.push(option);
            continue;
        };
        existing.expects_value |= option.expects_value;
        if existing.description.is_none() {
            existing.description = option.description;
        }
        if existing.values.is_empty() {
            existing.values = option.values;
        }
        for name in option.names {
            if !existing.names.contains(&name) {
                existing.names.push(name);
            }
        }
    }
    merged
}

pub(crate) fn parse_positionals(rows: &[Row]) -> Vec<HelpPositional> {
    rows.iter()
        .filter_map(|row| {
            let name = row
                .left
                .split_whitespace()
                .next()?
                .trim_matches('.')
                .trim_matches(['<', '>', '[', ']']);
            (!name.is_empty()).then(|| HelpPositional {
                name: name.to_string(),
                description: clean_description(&row.description),
                kind: argument_kind(name),
            })
        })
        .collect()
}

fn option_names(left: &str) -> Vec<String> {
    left.split(',')
        .flat_map(|part| part.split('|'))
        .filter_map(|part| {
            part.split_whitespace()
                .find(|value| value.starts_with('-'))
                .map(|value| {
                    value
                        .split('=')
                        .next()
                        .unwrap_or(value)
                        .trim_end_matches('.')
                        .to_string()
                })
        })
        .collect()
}

pub(super) fn placeholder(value: &str) -> bool {
    value.starts_with('<') || value.starts_with('[')
}

pub(super) fn argument_kind(value: &str) -> ArgumentKind {
    let value = value.to_ascii_lowercase();
    if value.contains("url") || value.contains("uri") || value.contains("link") {
        ArgumentKind::Url
    } else if value.contains("dir") || value.contains("folder") || value.contains("project") {
        ArgumentKind::Directory
    } else if value.contains("file") || value.contains("path") {
        ArgumentKind::File
    } else if value.contains("number") || value.contains("port") {
        ArgumentKind::Number
    } else if value.contains("message")
        || value.contains("text")
        || value.contains("prompt")
        || value.contains("module")
        || value.contains("provider")
        || value.contains("target")
    {
        ArgumentKind::Text
    } else {
        ArgumentKind::Value
    }
}

pub(super) fn bracket_value<'a>(text: &'a str, label: &str) -> Option<&'a str> {
    let start = text.to_ascii_lowercase().find(&format!("[{label}"))?;
    let value_start = start + label.len() + 1;
    let end = text[value_start..].find(']')? + value_start;
    Some(text[value_start..end].trim())
}

fn parse_values(value: &str) -> Vec<String> {
    value
        .split([',', '|'])
        .map(str::trim)
        .map(|value| value.trim_matches(['\'', '"']))
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn inline_values(text: &str) -> Option<Vec<String>> {
    [('{', '}'), ('<', '>')]
        .into_iter()
        .find_map(|(open, close)| {
            let start = text.find(open)?;
            let end = text[start + 1..].find(close)? + start + 1;
            let value = &text[start + 1..end];
            (value.contains(['|', ','])).then(|| parse_values(value))
        })
}

fn has_value_tag(text: &str) -> bool {
    [
        "[string]",
        "[number]",
        "[array]",
        "[choices:",
        "[possible values:",
    ]
    .iter()
    .any(|tag| text.to_ascii_lowercase().contains(tag))
}

fn option_has_argument(left: &str) -> bool {
    left.split_whitespace()
        .skip(1)
        .any(|token| !token.starts_with('-') && !matches!(token, ":" | "..."))
}

pub(super) fn clean_description(text: &str) -> Option<String> {
    let mut value = text.trim().to_string();
    while let Some(start) = value.find(" [") {
        let Some(_end) = value[start + 2..].find(']') else {
            break;
        };
        value.truncate(start);
    }
    let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    (!value.is_empty()).then_some(value)
}
