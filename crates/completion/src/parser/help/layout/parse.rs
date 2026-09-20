use super::{
    Document, Row, Section, options, rows,
    rows::{
        append_description, command_description_continuation, command_grid, row_allowed,
        split_columns, unlabelled_row,
    },
    sections, tree,
};

pub(super) fn extract(text: &str) -> Document {
    let mut document = Document {
        preamble: Vec::new(),
        usage: Vec::new(),
        commands: Vec::new(),
        options: Vec::new(),
        positionals: Vec::new(),
    };
    let mut section = Section::Other;
    let mut usage_open = false;
    let mut usage_header_empty = false;
    let mut usage_seen = false;
    let mut tree_state = tree::State::new();
    let mut command_row_seen = false;
    let mut command_group = false;
    let mut preamble = true;
    for raw in text.lines().map(str::trim_end) {
        if let Some(rest) = sections::usage_line(raw) {
            preamble = false;
            usage_open = true;
            usage_header_empty = rest.trim().is_empty();
            usage_seen = !rest.trim().is_empty();
            command_group = false;
            append_usage(&mut document.usage, rest, true);
            continue;
        }
        if let Some(next) = sections::section_header(raw) {
            preamble = false;
            usage_open = false;
            usage_header_empty = false;
            usage_seen = false;
            tree_state.reset();
            command_row_seen = false;
            command_group = sections::is_command_group_header(raw);
            section = next;
            continue;
        }
        if usage_open {
            if raw.trim().is_empty() {
                usage_open = usage_header_empty && !usage_seen;
                continue;
            }
            if raw.starts_with([' ', '\t']) {
                let value = raw.trim();
                let new_line = usage_starts_new_form(&document.usage, value);
                append_usage(&mut document.usage, value, new_line);
                usage_seen = true;
                continue;
            }
            if !raw.trim().is_empty() && sections::looks_like_usage(raw) {
                append_usage(&mut document.usage, raw.trim(), true);
                usage_seen = true;
                continue;
            }
            usage_open = false;
            usage_header_empty = false;
            usage_seen = false;
        }
        if raw.trim().is_empty() {
            continue;
        }
        if preamble {
            document.preamble.push(raw.trim().to_string());
        }
        append_row(
            &mut document,
            &mut tree_state,
            &mut command_row_seen,
            command_group,
            section,
            raw,
        );
    }
    document
}

fn append_row(
    document: &mut Document,
    tree_state: &mut tree::State,
    command_row_seen: &mut bool,
    command_group: bool,
    section: Section,
    raw: &str,
) -> bool {
    if section == Section::Commands && tree_state.append(document, raw) {
        return true;
    }
    if section == Section::Other
        && command_group
        && let Some((left, description)) = rows::command_table_row(raw)
    {
        tree_state.reset();
        push_row(document, Section::Commands, raw, left, description);
        *command_row_seen = true;
        return true;
    }
    if section == Section::Other
        && command_group
        && *command_row_seen
        && let Some(row) = document.commands.last_mut()
        && indentation(raw) > row.indent
    {
        append_description(row, raw);
        return true;
    }
    if section == Section::Commands
        && !*command_row_seen
        && !rows::command_row(raw)
        && let Some(row) = document.options.last_mut()
        && indentation(raw) > row.indent
    {
        append_description(row, raw);
        return true;
    }
    if section == Section::Commands && options::append(document, raw) {
        return true;
    }
    if section == Section::Commands
        && command_description_continuation(raw, document.commands.last())
    {
        let Some(row) = document.commands.last_mut() else {
            return false;
        };
        append_description(row, raw);
        return true;
    }
    if section == Section::Commands
        && let Some(names) = command_grid(raw)
    {
        tree_state.reset();
        for name in names {
            document.commands.push(Row {
                indent: indentation(raw),
                left: name.clone(),
                description: String::new(),
                columns: vec![name.clone()],
                children: Vec::new(),
            });
        }
        *command_row_seen = true;
        return true;
    }
    if matches!(section, Section::Options | Section::Other) && options::append(document, raw) {
        return true;
    }
    if is_new_row(document, section, raw)
        && let Some((left, description)) =
            split_columns(raw).or_else(|| unlabelled_row(raw, section))
        && row_allowed(section, &left)
    {
        if section == Section::Commands {
            tree_state.reset();
            *command_row_seen = true;
        }
        push_row(document, section, raw, left, description);
        return true;
    }
    if section == Section::Commands && command_list(raw) {
        tree_state.reset();
        for name in raw.trim().trim_end_matches(',').split(',') {
            document.commands.push(Row {
                indent: indentation(raw),
                left: name.trim().to_string(),
                description: String::new(),
                columns: vec![name.trim().to_string()],
                children: Vec::new(),
            });
        }
        *command_row_seen = true;
        return true;
    }
    let Some(row) = last_row_mut(document, section) else {
        return false;
    };
    if !raw.starts_with([' ', '\t']) {
        return false;
    }
    if section == Section::Options && indentation(raw) <= row.indent {
        return false;
    }
    append_description(row, raw);
    true
}

fn push_row(
    document: &mut Document,
    section: Section,
    raw: &str,
    left: String,
    description: String,
) {
    let row = Row {
        indent: indentation(raw),
        left,
        description,
        columns: rows::columns(raw),
        children: Vec::new(),
    };
    match section {
        Section::Commands => document.commands.push(row),
        Section::Options => document.options.push(row),
        Section::Positionals => document.positionals.push(row),
        Section::Other => {}
    }
}

fn is_new_row(document: &Document, section: Section, raw: &str) -> bool {
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

fn indentation(value: &str) -> usize {
    value.len() - value.trim_start().len()
}

fn last_row_mut(document: &mut Document, section: Section) -> Option<&mut Row> {
    match section {
        Section::Commands => document.commands.last_mut(),
        Section::Options => document.options.last_mut(),
        Section::Positionals => document.positionals.last_mut(),
        Section::Other => None,
    }
}

fn command_list(line: &str) -> bool {
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

fn append_usage(target: &mut Vec<String>, value: &str, new_line: bool) {
    let value = usage_form(value);
    if value.is_empty() {
        return;
    }
    if new_line || target.is_empty() {
        target.push(value.to_string());
    } else if let Some(last) = target.last_mut() {
        if !last.is_empty() {
            last.push(' ');
        }
        last.push_str(value);
    }
}

fn usage_starts_new_form(existing: &[String], value: &str) -> bool {
    let Some(first) = existing
        .first()
        .and_then(|usage| usage.split_whitespace().next())
    else {
        return true;
    };
    usage_form(value).split_whitespace().next() == Some(first)
}

fn usage_form(value: &str) -> &str {
    value
        .trim()
        .strip_prefix("or:")
        .map_or(value.trim(), str::trim_start)
}
