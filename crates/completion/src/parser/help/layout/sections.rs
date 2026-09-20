#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Section {
    Commands,
    Options,
    Positionals,
    Other,
}

pub(super) fn usage_line(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    trimmed
        .strip_prefix("Usage:")
        .or_else(|| trimmed.strip_prefix("usage:"))
        .or_else(|| trimmed.strip_prefix("SYNOPSIS:"))
        .or_else(|| trimmed.strip_prefix("Synopsis:"))
        .or_else(|| trimmed.eq_ignore_ascii_case("synopsis").then_some(""))
}

pub(super) fn looks_like_usage(line: &str) -> bool {
    let value = line.trim();
    value.split_whitespace().count() > 1
        && !uppercase_heading(value)
        && value.contains(['[', '<', '{', '-'])
}

pub(super) fn section_header(line: &str) -> Option<Section> {
    let trimmed = line.trim();
    let name = trimmed.trim_end_matches(':').to_ascii_lowercase();
    let colon_heading = !line.starts_with([' ', '\t']) && line.trim_end().ends_with(':');
    let semantic = semantic_heading(&name)
        && ((!line.starts_with([' ', '\t']) && section_qualifier(&name))
            || (line.trim_end().ends_with(':') && section_qualifier(&name)));
    let table = !line.starts_with([' ', '\t']) && table_heading(&name);
    let prose_heading = title_heading(trimmed) && indentation(line) <= 4;
    if !(colon_heading
        || semantic
        || table
        || known_heading(&name)
        || uppercase_heading(trimmed)
        || prose_heading)
    {
        return None;
    }
    Some(if option_section(&name) {
        Section::Options
    } else if positional_section(&name) {
        Section::Positionals
    } else if command_section(&name) {
        Section::Commands
    } else {
        Section::Other
    })
}

pub(super) fn is_command_group_header(line: &str) -> bool {
    let trimmed = line.trim();
    let name = trimmed.trim_end_matches(':').trim().to_ascii_lowercase();
    !line.starts_with([' ', '\t'])
        && line.trim_end().ends_with(':')
        && !uppercase_heading(trimmed.trim_end_matches(':').trim())
        && !known_heading(&name)
}

fn known_heading(name: &str) -> bool {
    matches!(
        name,
        "commands"
            | "all commands"
            | "available commands"
            | "subcommands"
            | "synopsis"
            | "options"
            | "flags"
            | "arguments"
            | "positional arguments"
            | "operands"
    )
}

fn option_section(name: &str) -> bool {
    matches!(name, "options" | "flags" | "switches")
        || name.ends_with(" options")
        || name.ends_with(" flags")
}

fn positional_section(name: &str) -> bool {
    matches!(
        name,
        "arguments" | "positional arguments" | "operands" | "parameters"
    )
}

fn command_section(name: &str) -> bool {
    matches!(
        name,
        "commands" | "all commands" | "available commands" | "subcommands"
    ) || name.contains(" commands")
}

fn semantic_heading(name: &str) -> bool {
    known_heading(name)
        || section_qualifier(name)
            && (name.ends_with(" options")
                || name.ends_with(" flags")
                || name.ends_with(" arguments")
                || name.ends_with(" commands"))
}

fn table_heading(name: &str) -> bool {
    name.ends_with("commands")
        || name.ends_with("options")
        || name.ends_with("arguments")
        || name.ends_with("flags")
}

fn section_qualifier(name: &str) -> bool {
    matches!(
        name.split_whitespace().next(),
        Some(
            "advanced"
                | "common"
                | "global"
                | "general"
                | "input"
                | "output"
                | "other"
                | "per-file"
                | "per-stream"
                | "standard"
                | "where"
        )
    )
}

fn uppercase_heading(value: &str) -> bool {
    let mut has_letter = false;
    value.chars().all(|character| {
        if character.is_ascii_uppercase() {
            has_letter = true;
            true
        } else {
            character.is_ascii_whitespace() || matches!(character, '-' | '_')
        }
    }) && has_letter
}

fn title_heading(value: &str) -> bool {
    let words = value.split_whitespace().collect::<Vec<_>>();
    if words.is_empty() || value.ends_with([':', '.', ',', ';']) {
        return false;
    }
    words.iter().all(|word| {
        word.chars()
            .next()
            .is_some_and(|character| character.is_ascii_uppercase() || character.is_ascii_digit())
    })
}

fn indentation(value: &str) -> usize {
    value.len() - value.trim_start().len()
}
