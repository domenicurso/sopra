#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Section {
    Commands,
    Options,
    Positionals,
}

#[derive(Clone)]
pub(crate) struct Row {
    pub(crate) left: String,
    pub(crate) description: String,
}

pub(crate) fn rows(text: &str) -> (Vec<Row>, Vec<Row>, Vec<Row>) {
    let mut section = None;
    let mut commands = Vec::new();
    let mut options = Vec::new();
    let mut positionals = Vec::new();
    for line in text.lines().map(str::trim_end) {
        if line.trim().is_empty() || is_ignored(line) {
            continue;
        }
        if let Some(next) = section_header(line) {
            section = Some(next);
            continue;
        }
        let columns = split_columns(line).or_else(|| unlabelled_row(line, section));
        let candidate = columns.as_ref().is_some_and(|(left, _)| match section {
            Some(Section::Commands) => command_left(left),
            Some(Section::Options) => left.trim_start().starts_with('-'),
            Some(Section::Positionals) => positional_left(left),
            None => left.trim_start().starts_with('-'),
        });
        if let Some((left, description)) = columns.filter(|_| candidate) {
            let row = Row { left, description };
            match section.unwrap_or(Section::Options) {
                Section::Commands => commands.push(row),
                Section::Options => options.push(row),
                Section::Positionals => positionals.push(row),
            }
        } else if section == Some(Section::Commands) && command_list(line) {
            commands.extend(
                line.trim()
                    .trim_end_matches(',')
                    .split(',')
                    .map(|name| Row {
                        left: name.trim().to_string(),
                        description: String::new(),
                    }),
            );
        } else if let Some(description) = line.trim().strip_prefix('[') {
            append_continuation(
                section,
                description,
                &mut commands,
                &mut options,
                &mut positionals,
            );
        }
    }
    if options.is_empty() {
        options.extend(usage::rows(text));
    }
    if commands.is_empty() {
        commands.extend(usage::command_rows(text));
    }
    (commands, options, positionals)
}

fn section_header(line: &str) -> Option<Section> {
    let name = line.trim().trim_end_matches(':').to_ascii_lowercase();
    if !line.trim_end().ends_with(':') {
        return None;
    }
    if name.contains("command") || name.contains("subcommand") {
        Some(Section::Commands)
    } else if name.contains("option") || name.contains("flag") {
        Some(Section::Options)
    } else if name.contains("argument") || name.contains("positional") || name.contains("parameter")
    {
        Some(Section::Positionals)
    } else {
        None
    }
}

fn split_columns(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if let Some((left, right)) = trimmed.split_once('\t') {
        return Some((left.trim().to_string(), right.trim().to_string()));
    }
    let mut run_start = None;
    let mut run_length = 0;
    for (index, character) in trimmed.char_indices() {
        if character == ' ' {
            run_start.get_or_insert(index);
            run_length += 1;
        } else {
            if run_length >= 2 {
                let start = run_start.expect("space run has a start");
                return Some((
                    trimmed[..start].trim().to_string(),
                    trimmed[index..].trim().to_string(),
                ));
            }
            run_start = None;
            run_length = 0;
        }
    }
    None
}

fn unlabelled_row(line: &str, section: Option<Section>) -> Option<(String, String)> {
    let value = line.trim();
    let is_option = section == Some(Section::Options)
        && (value.starts_with('-') || value.starts_with('[') && value.contains('-'));
    let is_positional = section == Some(Section::Positionals)
        && value.starts_with(['<', '['])
        && !value.ends_with(':');
    (is_option || is_positional).then(|| (value.to_string(), String::new()))
}

fn append_continuation(
    section: Option<Section>,
    text: &str,
    commands: &mut [Row],
    options: &mut [Row],
    positionals: &mut [Row],
) {
    let rows = match section.unwrap_or(Section::Options) {
        Section::Commands => commands,
        Section::Options => options,
        Section::Positionals => positionals,
    };
    if let Some(row) = rows.last_mut() {
        row.description.push(' ');
        row.description.push_str(text.trim_end_matches(']'));
    }
}

fn command_left(left: &str) -> bool {
    left.split_whitespace()
        .next()
        .is_some_and(|value| !value.starts_with('-'))
}

fn positional_left(left: &str) -> bool {
    left.split_whitespace()
        .next()
        .is_some_and(|value| !value.starts_with('-'))
}

fn is_ignored(line: &str) -> bool {
    let value = line.trim().to_ascii_lowercase();
    value.starts_with("usage:") || value.starts_with("version:") || value == "synopsis"
}

fn command_list(line: &str) -> bool {
    let value = line.trim().trim_end_matches(',');
    value.contains(',')
        && value
            .split(',')
            .map(str::trim)
            .all(|name| !name.is_empty() && name.chars().all(command_name_char))
}

fn command_name_char(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
}
use super::usage;
