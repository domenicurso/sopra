use crate::graph::{CommandNode, evidence::Evidence};

// Evidence uses normalized help/rendered-man line numbers, the same text the parser consumes.
pub(super) fn annotate(node: &mut CommandNode, source: &str, text: &str) {
    let lines = text.lines().collect::<Vec<_>>();
    for option in &node.options {
        let observation = format!(
            "aliases={:?}; attachment={:?}; value={:?}; repeatable={}; description={:?}",
            option.names, option.attachment, option.value, option.repeatable, option.description
        );
        node.evidence
            .extend(records(&lines, source, &option.names, &observation));
    }
    for positional in &node.positionals {
        let observation = format!(
            "positional; kind={:?}; optional={}; repeatable={}; choices={:?}",
            positional.value.kind,
            positional.optional,
            positional.repeatable,
            positional.value.choices
        );
        node.evidence.extend(records(
            &lines,
            source,
            std::slice::from_ref(&positional.name),
            &observation,
        ));
    }
    for child in &mut node.subcommands {
        let observation = format!(
            "command; aliases={:?}; description={:?}",
            child.aliases, child.description
        );
        let names = std::iter::once(child.name.clone())
            .chain(child.aliases.iter().cloned())
            .collect::<Vec<_>>();
        node.evidence
            .extend(records(&lines, source, &names, &observation));
        annotate(child, source, text);
    }
}

fn records(lines: &[&str], source: &str, names: &[String], observation: &str) -> Vec<Evidence> {
    let mut evidence = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        for name in names {
            if mentions(line, name) {
                evidence.push(Evidence {
                    target: name.clone(),
                    source: source.into(),
                    line: index + 1,
                    text: excerpt(lines, index),
                    observation: observation.into(),
                });
            }
        }
    }
    evidence
}

fn mentions(line: &str, name: &str) -> bool {
    line.split(|character: char| character.is_whitespace() || "[],=<>|{}()".contains(character))
        .any(|token| token == name || token.trim_end_matches(':') == name.trim_end_matches(':'))
}

fn excerpt(lines: &[&str], index: usize) -> String {
    let indent = lines[index].len() - lines[index].trim_start().len();
    let mut selected = vec![lines[index]];
    for line in lines.iter().skip(index + 1).take(12) {
        if line.trim().is_empty() || line.len() - line.trim_start().len() <= indent {
            break;
        }
        selected.push(line);
    }
    selected.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alias_and_wrapped_description_evidence_keeps_source_line_numbers() {
        let text = "Options:\n  -m, --mode <fast|slow>\n        Choose execution mode\n";
        let mut graph = crate::parser::help::parse(text, "fixture");
        annotate(&mut graph.root, "help source 1", text);
        let evidence = graph
            .root
            .evidence
            .iter()
            .find(|entry| entry.target == "--mode")
            .expect("alias evidence");
        assert_eq!(evidence.line, 2);
        assert_eq!(evidence.source, "help source 1");
        assert!(evidence.text.contains("Choose execution mode"));
        assert!(evidence.observation.contains("fast"));
    }
}
