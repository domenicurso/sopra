use crate::{graph::CommandNode, inspection::GraphQuery};
use std::fmt::Write;

pub(crate) fn explain(query: &GraphQuery<'_>, entry: &str) -> Result<String, String> {
    let request = super::dump::request(query)?;
    let graph = super::discover::resolve_chunk(&request)
        .ok_or_else(|| format!("no completion data found for: {}", query.command))?;
    let mut output = selected(&graph.root, entry)
        .ok_or_else(|| format!("no graph entry named {entry:?} in this command path"))?;
    let evidence = graph
        .root
        .evidence
        .iter()
        .filter(|evidence| evidence.target == entry)
        .collect::<Vec<_>>();
    if evidence.is_empty() {
        output.push_str(
            "Evidence: no direct source line matched; inferred from synopsis structure\n",
        );
    }
    for evidence in evidence {
        let _ = writeln!(
            output,
            "\n{}:{}\n{}\nParsed: {}",
            evidence.source, evidence.line, evidence.text, evidence.observation
        );
    }
    output.push_str("\nMerge: help summaries precede manual paragraphs; aliases and choices accumulate; optionality requires agreement.\n");
    Ok(output)
}

fn selected(node: &CommandNode, entry: &str) -> Option<String> {
    if let Some(option) = node
        .options
        .iter()
        .find(|option| option.names.iter().any(|name| name == entry))
    {
        return Some(format!(
            "Option: {}\nSelected description: {}\nAttachment: {:?}\nValue: {:?}\nRepeatable: {}\nExclusive groups: {:?}\n",
            option.names.join(", "),
            option.description.as_deref().unwrap_or("-"),
            option.attachment,
            option.value,
            option.repeatable,
            node.exclusive
                .iter()
                .filter(|group| group.iter().any(|name| option.names.contains(name)))
                .collect::<Vec<_>>()
        ));
    }
    if let Some(child) = node
        .subcommands
        .iter()
        .find(|child| child.name == entry || child.aliases.iter().any(|alias| alias == entry))
    {
        return Some(format!(
            "Command: {}\nSelected description: {}\nAliases: {:?}\n",
            child.name,
            child.description.as_deref().unwrap_or("-"),
            child.aliases
        ));
    }
    node.positionals
        .iter()
        .find(|positional| positional.name == entry)
        .map(|positional| {
            format!(
                "Positional: {}\nValue: {:?}\nOptional: {}\nRepeatable: {}\n",
                positional.name, positional.value, positional.optional, positional.repeatable
            )
        })
}
