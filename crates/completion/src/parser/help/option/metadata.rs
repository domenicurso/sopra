pub(crate) fn clean_description(description: &str) -> Option<String> {
    let value = strip_metadata(description)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    (!value.is_empty()).then_some(value)
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

pub(super) fn repeatable(description: &str) -> bool {
    let lower = description.to_ascii_lowercase();
    ["multiple times", "more than once", "repeatable"]
        .iter()
        .any(|phrase| lower.contains(phrase))
}
