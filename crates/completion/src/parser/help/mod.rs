pub(crate) mod gnu;
pub(crate) mod posix;

mod layout;
mod option;

use super::normalize::terminal_text;
use crate::graph::{CommandGraph, CommandNode};

pub(crate) use layout::Document;

pub(crate) fn matches_path(text: &str, program: &str, path: &[String]) -> bool {
    if path.is_empty() {
        return true;
    }
    let document = layout::extract(&terminal_text(text));
    document
        .usage
        .iter()
        .any(|usage| starts_with_path(usage, program, path))
        || document
            .commands
            .iter()
            .any(|row| starts_with_path(&row.left, program, path))
        || (document.usage.is_empty() && document.commands.is_empty())
}

fn starts_with_path(value: &str, program: &str, path: &[String]) -> bool {
    let mut tokens = value.split_whitespace();
    std::iter::once(program)
        .chain(path.iter().map(String::as_str))
        .all(|expected| tokens.next() == Some(expected))
}

#[cfg(test)]
pub(crate) fn parse(text: &str, program: &str) -> CommandGraph {
    parse_for(text, program, &[])
}

#[cfg(test)]
pub(crate) fn parse_for(text: &str, program: &str, path: &[String]) -> CommandGraph {
    parse_for_with(text, program, path, true)
}

pub(crate) fn parse_for_with(
    text: &str,
    program: &str,
    path: &[String],
    discover_usage_commands: bool,
) -> CommandGraph {
    let document = layout::extract(&terminal_text(text));
    let mut root = CommandNode::named(program);
    root.description = preamble_description(&document, program, path);
    gnu::parse(&document, &mut root, path);
    posix::parse(&document, &mut root, path, discover_usage_commands);
    CommandGraph { root }
}

fn preamble_description(document: &Document, program: &str, path: &[String]) -> Option<String> {
    let title = std::iter::once(program)
        .chain(path.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ");
    document
        .preamble
        .iter()
        .find_map(|line| clean_preamble_line(line, program, &title))
}

fn clean_preamble_line(line: &str, program: &str, title: &str) -> Option<String> {
    let mut value = line.trim();
    let error_prefix = format!("{program} error ");
    if let Some(rest) = value.strip_prefix(&error_prefix) {
        value = rest.trim();
    }
    if value.is_empty()
        || value.eq_ignore_ascii_case(title)
        || value.eq_ignore_ascii_case("usage")
        || value.starts_with("Usage:")
        || value.starts_with("usage:")
        || value.starts_with("error:")
        || value.starts_with("warning:")
        || looks_like_completion_source(value)
        || !value.chars().any(char::is_alphabetic)
        || looks_like_invocation(value, program)
    {
        return None;
    }
    option::clean_description(value)
}

fn looks_like_completion_source(value: &str) -> bool {
    (value.starts_with('#') && value.to_ascii_lowercase().contains("completion"))
        || value.starts_with("#compdef")
        || value.starts_with("complete ")
        || value.starts_with("if ")
        || value.starts_with("case ")
        || value.starts_with("function ")
        || value.contains("; then")
        || value.contains("() {")
        || value.contains("-begin-")
        || value.contains("-end-")
}

fn looks_like_invocation(value: &str, program: &str) -> bool {
    let mut tokens = value.split_whitespace();
    if tokens.next() != Some(program) {
        return false;
    }
    tokens.any(|token| token.starts_with(['<', '[', '{']))
}

#[cfg(test)]
mod tests;
