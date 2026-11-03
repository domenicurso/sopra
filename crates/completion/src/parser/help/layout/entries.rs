use super::{Document, Row, Section, rows};

pub(super) fn push_row(
    document: &mut Document,
    section: Section,
    raw: &str,
    columns: (String, String),
) {
    let fields = rows::columns(raw);
    let fields = if fields.len() < 2 && !columns.1.is_empty() {
        vec![columns.0.clone(), columns.1.clone()]
    } else {
        fields
    };
    let row = Row {
        indent: indentation(raw),
        left: columns.0,
        description: columns.1,
        columns: fields,
        children: Vec::new(),
    };
    match section {
        Section::Commands => document.commands.push(row),
        Section::Options => document.options.push(row),
        Section::Positionals => document.positionals.push(row),
        Section::Other => {}
    }
}

pub(super) fn is_new_row(document: &Document, section: Section, raw: &str) -> bool {
    if section == Section::Commands {
        return document
            .commands
            .last()
            .is_none_or(|row| indentation(raw) <= row.indent);
    }
    if section != Section::Options {
        return true;
    }
    if raw.trim_start().starts_with('-') {
        let previous = document.options.last();
        return previous.is_none_or(|row| indentation(raw) <= row.indent);
    }
    let previous = match section {
        Section::Commands => document.commands.last(),
        Section::Options => document.options.last(),
        Section::Positionals => document.positionals.last(),
        Section::Other => None,
    };
    previous.is_none_or(|row| indentation(raw) <= row.indent)
}

pub(super) fn indentation(value: &str) -> usize {
    value.len() - value.trim_start().len()
}

pub(super) fn last_row_mut(document: &mut Document, section: Section) -> Option<&mut Row> {
    match section {
        Section::Commands => document.commands.last_mut(),
        Section::Options => document.options.last_mut(),
        Section::Positionals => document.positionals.last_mut(),
        Section::Other => None,
    }
}

pub(super) fn command_list(line: &str) -> bool {
    let value = line.trim().trim_end_matches(',');
    value.contains(',')
        && value.split(',').all(|name| {
            let name = name.trim();
            !name.is_empty()
                && name.chars().all(|character| {
                    character.is_ascii_alphanumeric() || "_-.:".contains(character)
                })
        })
}

pub(super) fn continuation(document: &mut Document, section: Section, raw: &str) -> bool {
    let Some(row) = last_row_mut(document, section) else {
        return false;
    };
    if !raw.starts_with([' ', '\t']) {
        return false;
    }
    if indentation(raw) <= row.indent {
        return false;
    }
    rows::append_description(row, raw);
    true
}

pub(super) fn command_context(document: &Document) -> bool {
    document.usage.is_empty()
        || document.usage.iter().any(|usage| {
            usage.split_whitespace().any(|token| {
                let name = token
                    .trim_matches(['[', ']', '<', '>', '{', '}'])
                    .to_ascii_lowercase();
                matches!(name.as_str(), "command" | "subcommand" | "cmd" | "action")
            })
        })
}
