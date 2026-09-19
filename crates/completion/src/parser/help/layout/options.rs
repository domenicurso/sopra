use super::super::option;
use super::{Document, Row, Section, rows};

pub(super) fn append(document: &mut Document, raw: &str) -> bool {
    let first = raw.split_whitespace().next().unwrap_or_default();
    if option::option_token(first).is_none() {
        return false;
    }
    if document
        .options
        .last()
        .is_some_and(|row| raw.len() - raw.trim_start().len() > row.indent.saturating_add(4))
    {
        return false;
    }
    let columns = rows::columns(raw);
    if columns.len() < 2 && !raw.starts_with([' ', '\t']) {
        return false;
    }
    let first_column = columns.first().cloned().unwrap_or_default();
    let (left, first_description) = option_prefix(&first_column);
    let description = std::iter::once(first_description)
        .chain(columns.iter().skip(1).cloned())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if !rows::row_allowed(Section::Options, &left) {
        return false;
    }
    document.options.push(Row {
        indent: raw.len() - raw.trim_start().len(),
        left,
        description,
        columns: rows::columns(raw),
        children: Vec::new(),
    });
    true
}

fn option_prefix(value: &str) -> (String, String) {
    let tokens = value.split_whitespace().collect::<Vec<_>>();
    let mut split = 0;
    for (index, token) in tokens.iter().enumerate() {
        let token = token.trim_matches(',');
        if option::is_option_token(token)
            || (index > 0 && option::placeholder(token))
            || token.starts_with('=')
        {
            split = index + 1;
        } else {
            break;
        }
    }
    (tokens[..split].join(" "), tokens[split..].join(" "))
}
