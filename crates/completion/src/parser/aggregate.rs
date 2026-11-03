use super::{cache::Source, discover::HelpRequest, help, man};
use crate::graph::{CommandGraph, CommandNode};

pub(super) fn aggregate(
    request: &HelpRequest,
    sources: &[Source],
    discover_usage_commands: bool,
) -> Option<CommandGraph> {
    let mut graph: Option<CommandGraph> = None;
    for (index, source) in sources.iter().enumerate() {
        let mut fragment = match source {
            Source::Help(text) if help::matches_path(text, &request.program, &request.args) => {
                help::parse_for_with(
                    text,
                    &request.program,
                    &request.args,
                    discover_usage_commands,
                )
            }
            Source::Help(_) => continue,
            Source::Man(text) => man::parse_for_with(
                text,
                &request.program,
                &request.args,
                discover_usage_commands,
            ),
        };
        if !fragment.root.useful() {
            continue;
        }
        let (label, text) = match source {
            Source::Help(text) => (format!("help source {}", index + 1), text.clone()),
            Source::Man(text) => (
                format!("rendered man source {}", index + 1),
                man::help_text(text),
            ),
        };
        super::evidence::annotate(&mut fragment.root, &label, &text);
        if let Some(current) = &mut graph {
            if matches!(source, Source::Man(_)) {
                preserve_help_summaries(&current.root, &mut fragment.root);
            }
            current.merge(fragment);
        } else {
            graph = Some(fragment);
        }
    }
    graph
}

// Help summaries fit menus; manuals supplement missing fields without replacing them with paragraphs.
fn preserve_help_summaries(current: &CommandNode, incoming: &mut CommandNode) {
    if current.description.is_some() {
        incoming.description = None;
    }
    for option in &mut incoming.options {
        if current.options.iter().any(|existing| {
            existing.description.is_some()
                && existing
                    .names
                    .iter()
                    .any(|name| option.names.contains(name))
        }) {
            option.description = None;
        }
    }
    for child in &mut incoming.subcommands {
        if let Some(existing) = current
            .subcommands
            .iter()
            .find(|existing| existing.name == child.name)
        {
            preserve_help_summaries(existing, child);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_evidence_fills_gaps_while_preserving_published_help_summaries() {
        let mut help = help::parse(
            "Commands:\n  build  Build source\n\nOptions:\n  --mode <kind>  Select mode\n",
            "tool",
        );
        let mut manual = help::parse(
            "Commands:\n  build  Build source with a very long explanation of compilation internals\n  test   Run the tests\n\nOptions:\n  --mode <kind>  Select a compilation mode [possible values: fast, slow]\n  --verbose  Show detail\n",
            "tool",
        );
        preserve_help_summaries(&help.root, &mut manual.root);
        help.merge(manual);
        assert_eq!(
            help.root.subcommands[0].description.as_deref(),
            Some("Build source")
        );
        assert_eq!(help.root.subcommands[1].name, "test");
        assert_eq!(
            help.root.options[0].description.as_deref(),
            Some("Select mode")
        );
        assert_eq!(
            help.root.options[0]
                .value
                .as_ref()
                .expect("mode takes a value")
                .choices,
            ["fast", "slow"]
        );
        assert_eq!(help.root.options[1].names, ["--verbose"]);
    }
}
