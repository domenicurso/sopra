use super::{
    help,
    help_pages::{self, Page},
};
use std::{ffi::OsString, io};

const GRAPH_OPTIONS: &[(&str, &str)] = &[
    (
        "--fresh",
        "Bypass the source cache and replace it with new probe output",
    ),
    ("--root", "Inspect the command and its immediate children"),
    ("--chunk", "Inspect one nested command path"),
    (
        "--cwd <directory>",
        "Resolve and probe commands in this directory",
    ),
];

struct Topic {
    usage: String,
    commands: Vec<(&'static str, &'static str)>,
    notes: Vec<&'static str>,
}

pub(super) fn print_requested(arguments: &[OsString]) -> io::Result<bool> {
    let Some(words) = arguments
        .iter()
        .map(|word| word.to_str())
        .collect::<Option<Vec<_>>>()
    else {
        return Ok(false);
    };
    let path = match words.as_slice() {
        ["help", path @ ..] => path,
        [path @ .., "--help" | "-h"] => path,
        _ => return Ok(false),
    };
    let Some(topic) = resolve(path) else {
        return Ok(false);
    };
    help::print(&Page {
        usage: &topic.usage,
        commands: &topic.commands,
        notes: &topic.notes,
    })?;
    Ok(true)
}

fn resolve(path: &[&str]) -> Option<Topic> {
    let (page, prefix, rest) = match path {
        ["debug", "graph", rest @ ..] => (&help_pages::GRAPH, "debug graph", rest),
        ["dev", rest @ ..] => (&help_pages::DEV, "dev", rest),
        ["debug", rest @ ..] => (&help_pages::DEBUG, "debug", rest),
        _ => (&help_pages::ROOT, "", path),
    };
    if rest.is_empty() {
        return Some(Topic {
            usage: page.usage.into(),
            commands: page.commands.to_vec(),
            notes: page.notes.to_vec(),
        });
    }
    let name = rest.join(" ");
    if let Some(topic) = leaf(page, prefix, rest) {
        return Some(topic);
    }
    let prefix = format!("{name} ");
    let commands = page
        .commands
        .iter()
        .filter_map(|(signature, description)| {
            signature
                .strip_prefix(&prefix)
                .map(|name| (name, *description))
        })
        .collect::<Vec<_>>();
    (!commands.is_empty()).then(|| Topic {
        usage: format!("Usage: sopra {name} <command>"),
        commands,
        notes: Vec::new(),
    })
}

fn leaf(page: &Page<'static>, prefix: &str, rest: &[&str]) -> Option<Topic> {
    if let Some((signature, description)) = page.commands.iter().find(|(signature, _)| {
        signature
            .split_whitespace()
            .take(rest.len())
            .eq(rest.iter().copied())
    }) {
        let arguments = signature.split_whitespace().count() > rest.len();
        if !arguments
            || signature
                .split_whitespace()
                .nth(rest.len())
                .is_some_and(|token| token.starts_with(['<', '[']))
        {
            return Some(Topic {
                usage: format!(
                    "Usage: sopra {}{signature}",
                    if prefix.is_empty() {
                        String::new()
                    } else {
                        format!("{prefix} ")
                    }
                ),
                commands: leaf_options(prefix, rest),
                notes: vec![description],
            });
        }
    }
    None
}

fn leaf_options(prefix: &str, rest: &[&str]) -> Vec<(&'static str, &'static str)> {
    if (prefix == "debug" && rest == ["completions"])
        || (prefix == "debug graph"
            && matches!(
                rest,
                ["generate" | "sources" | "cache" | "json" | "explain"]
            ))
    {
        GRAPH_OPTIONS.to_vec()
    } else if prefix == "debug" && rest == ["candidates"] {
        vec![
            (
                "--cwd <directory>",
                "Resolve commands and values in this directory",
            ),
            (
                "--cursor <number>",
                "Character offset within the supplied shell line",
            ),
        ]
    } else if prefix == "debug graph" && rest == ["parse"] {
        vec![(
            "--man",
            "Parse manual page text or roff instead of help output",
        )]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_describes_the_requested_path_instead_of_repeating_the_parent() {
        for path in [
            vec!["dev", "build"],
            vec!["dev", "link"],
            vec!["debug", "syntax"],
            vec!["config", "check"],
            vec!["widgets", "list"],
        ] {
            let topic = resolve(&path).expect("documented command");
            assert!(topic.usage.contains(&path.join(" ")));
            assert!(topic.commands.is_empty());
        }
        let topic = resolve(&["debug", "completions"]).expect("completion debugger");
        assert!(
            topic
                .commands
                .iter()
                .any(|(name, _)| *name == "--cwd <directory>")
        );
        assert!(resolve(&["dev", "unknown"]).is_none());
    }

    #[test]
    fn groups_are_derived_from_the_same_command_catalog_as_root_help() {
        assert_eq!(
            resolve(&["config"]).expect("config group").commands.len(),
            4
        );
        assert_eq!(
            resolve(&["widgets"]).expect("widget group").commands[0].0,
            "list"
        );
        assert_eq!(resolve(&["dev"]).expect("dev group").commands.len(), 7);
    }
}
