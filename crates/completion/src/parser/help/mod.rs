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
    gnu::parse(&document, &mut root, path);
    posix::parse(&document, &mut root, path, discover_usage_commands);
    CommandGraph { root }
}

#[cfg(test)]
mod tests;
