use std::fmt::Write;

use super::{CommandGraph, CommandNode, OptionSpec, PositionalSpec, ValueSpec};

pub(crate) fn render(graph: &CommandGraph) -> String {
    let mut output = String::new();
    render_node(&mut output, &graph.root, 0);
    output
}

fn render_node(output: &mut String, node: &CommandNode, depth: usize) {
    line(output, depth, &format!("command {}", node.name));
    if !node.aliases.is_empty() {
        line(
            output,
            depth + 1,
            &format!("aliases: {}", node.aliases.join(", ")),
        );
    }
    if let Some(description) = &node.description {
        line(output, depth + 1, &format!("description: {description}"));
    }
    if let Some(synopsis) = &node.synopsis {
        line(output, depth + 1, &format!("synopsis: {synopsis:?}"));
    }
    render_options(output, &node.options, depth);
    render_positionals(output, &node.positionals, depth);
    if !node.subcommands.is_empty() {
        line(output, depth + 1, "subcommands:");
        for child in &node.subcommands {
            render_node(output, child, depth + 2);
        }
    }
}

fn render_options(output: &mut String, options: &[OptionSpec], depth: usize) {
    if options.is_empty() {
        return;
    }
    line(output, depth + 1, "options:");
    for option in options {
        let names = option.names.join(", ");
        let value = option.value.as_ref().map(value_label).unwrap_or_default();
        let detail = option.description.as_deref().unwrap_or("-");
        line(
            output,
            depth + 2,
            &format!("{names}{value} [{:?}] {detail}", option.attachment),
        );
    }
}

fn render_positionals(output: &mut String, positionals: &[PositionalSpec], depth: usize) {
    if positionals.is_empty() {
        return;
    }
    line(output, depth + 1, "positionals:");
    for positional in positionals {
        let detail = positional.description.as_deref().unwrap_or("-");
        line(
            output,
            depth + 2,
            &format!(
                "{} {} {detail}",
                positional.name,
                value_label(&positional.value)
            ),
        );
    }
}

fn value_label(value: &ValueSpec) -> String {
    let choices = if value.choices.is_empty() {
        String::new()
    } else {
        format!(" choices={}", value.choices.join("|"))
    };
    format!(
        " <{}>{choices}",
        format!("{:?}", value.kind).to_ascii_lowercase()
    )
}

fn line(output: &mut String, depth: usize, value: &str) {
    let _ = writeln!(output, "{}{}", "  ".repeat(depth), value);
}
