use super::Row;
use super::Section;

pub(super) fn command_description_continuation(line: &str, previous: Option<&Row>) -> bool {
    let value = line.trim();
    previous.is_some_and(|row| !row.description.is_empty())
        && !line.starts_with([' ', '\t'])
        && value.split_whitespace().count() == 1
        && value.ends_with(['.', ',', ';', ':'])
}

pub(super) fn command_grid(line: &str) -> Option<Vec<String>> {
    let value = line.trim();
    let tokens = value.split_whitespace().collect::<Vec<_>>();
    if tokens.len() < 3 {
        return None;
    }
    let mut separators = 0;
    let mut run = 0;
    for character in value.chars() {
        if character == ' ' {
            run += 1;
        } else {
            if run >= 2 {
                separators += 1;
            }
            run = 0;
        }
    }
    (separators >= 3
        && tokens.iter().all(|token| {
            token
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "_-.:@/".contains(character))
        }))
    .then(|| tokens.into_iter().map(ToOwned::to_owned).collect())
}

pub(super) fn command_row(line: &str) -> bool {
    let first = line.split_whitespace().next().unwrap_or_default();
    !first.starts_with('-') && (split_columns(line).is_some() || manual_reference(first))
}

pub(super) fn split_columns(line: &str) -> Option<(String, String)> {
    let mut columns = columns(line);
    (columns.len() >= 2).then(|| {
        let description = columns.pop().unwrap_or_default();
        (columns.join(" "), description)
    })
}

pub(super) fn columns(line: &str) -> Vec<String> {
    let value = line.trim();
    let mut result = Vec::new();
    let mut field_start = 0;
    let mut gap_start = None;
    let mut gap_width = 0;
    for (index, character) in value.char_indices() {
        if character == ' ' {
            gap_start.get_or_insert(index);
            gap_width += 1;
            continue;
        }
        if character == '\t' {
            push_column(&mut result, value, field_start, gap_start.unwrap_or(index));
            field_start = index + character.len_utf8();
            gap_start = None;
            gap_width = 0;
            continue;
        }
        if gap_width >= 2 {
            let split = gap_start.unwrap_or(index);
            push_column(&mut result, value, field_start, split);
            field_start = index;
        }
        gap_start = None;
        gap_width = 0;
    }
    if gap_width >= 2 {
        push_column(
            &mut result,
            value,
            field_start,
            gap_start.unwrap_or(value.len()),
        );
        field_start = value.len();
    }
    push_column(&mut result, value, field_start, value.len());
    result
}

fn push_column(result: &mut Vec<String>, value: &str, start: usize, end: usize) {
    let column = value[start..end].trim();
    if !column.is_empty() {
        result.push(column.to_string());
    }
}

fn manual_reference(value: &str) -> bool {
    let Some(start) = value.rfind('(') else {
        return false;
    };
    value.ends_with(')')
        && !value[..start].is_empty()
        && value[start + 1..value.len() - 1]
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
}

pub(super) fn unlabelled_row(line: &str, section: Section) -> Option<(String, String)> {
    let value = line.trim();
    let first = value.split_whitespace().next()?;
    let allowed = match section {
        Section::Commands => value.split_whitespace().count() == 1 && !first.starts_with('-'),
        Section::Options => first.starts_with('-'),
        Section::Positionals => first.starts_with(['<', '[', '{']),
        Section::Other => false,
    };
    allowed.then(|| (value.to_string(), String::new()))
}

pub(super) fn row_allowed(section: Section, left: &str) -> bool {
    let first = left.split_whitespace().next().unwrap_or_default();
    match section {
        Section::Commands => !first.starts_with('-'),
        Section::Options => first.starts_with('-') || left.contains(" -"),
        Section::Positionals => !first.starts_with('-'),
        Section::Other => false,
    }
}

pub(super) fn append_description(row: &mut Row, raw: &str) {
    let continuation = clean(raw);
    if continuation.is_empty() {
        return;
    }
    if !row.description.is_empty() {
        row.description.push(' ');
    }
    row.description.push_str(&continuation);
}

fn clean(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}
