mod roff;

use crate::graph::CommandGraph;

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
    let source = if roff::looks_like_source(text) {
        roff::to_help_text(text)
    } else {
        text.to_string()
    };
    super::help::parse_for_with(&source, program, path, discover_usage_commands)
}
