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

pub(super) fn choices(text: &str) -> Vec<String> {
    let inline = text.trim().trim_matches(['<', '>', '[', ']', '{', '}']);
    if inline.contains(['|', ',']) {
        let values = parse_choices(inline);
        if values.len() > 1 && values.iter().all(|value| simple_choice(value)) {
            return values;
        }
    }
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
    bracketed_choices(text)
}

fn bracketed_choices(text: &str) -> Vec<String> {
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

pub(super) fn simple_choice(value: &str) -> bool {
    value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "._+:/-".contains(character))
}
