use super::super::layout::Row;

pub(super) fn signature(row: &Row) -> String {
    let Some(first) = row.columns.first() else {
        return row.left.clone();
    };
    let mut fields = vec![first.clone()];
    for column in row.columns.iter().skip(1) {
        if argument_column(column) {
            fields.push(column.clone());
        } else {
            break;
        }
    }
    fields.join(" ")
}

pub(super) fn argument_column(value: &str) -> bool {
    let first = value.split_whitespace().next().unwrap_or_default();
    first.starts_with(['<', '[', '{', '-']) || first == "..."
}

pub(super) fn description_text(row: &Row) -> String {
    if row.columns.len() > 2
        && row
            .columns
            .last()
            .is_some_and(|value| metadata_column(value))
    {
        return row.columns[1..].join(" ");
    }
    row.description.clone()
}

pub(super) fn metadata_column(value: &str) -> bool {
    value.starts_with('[') && value.ends_with(']')
}

pub(super) fn aliases(description: &str) -> Vec<String> {
    let lower = description.to_ascii_lowercase();
    let Some(start) = lower.find("[aliases:") else {
        return Vec::new();
    };
    let start = start + "[aliases:".len();
    let end = description[start..]
        .find(']')
        .map_or(description.len(), |offset| start + offset);
    description[start..end]
        .split([',', '|', ' '])
        .map(str::trim)
        .filter(|alias| !alias.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub(super) fn choice_names(left: &str) -> Vec<String> {
    let Some(start) = left.find('{') else {
        return Vec::new();
    };
    let Some(end) = left[start + 1..].find('}') else {
        return Vec::new();
    };
    left[start + 1..start + 1 + end]
        .split([',', '|'])
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub(super) fn description_is_link(value: &str) -> bool {
    value
        .split_whitespace()
        .any(|token| token.starts_with("http://") || token.starts_with("https://"))
}
