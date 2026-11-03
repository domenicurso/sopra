use crate::{
    Request,
    engine::{arguments, candidates},
    filesystem::FilesystemEngine,
    parser::Invocation,
};

fn context(line: &str) -> (Request, Invocation) {
    let args = line
        .split_whitespace()
        .skip(1)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let trailing_space = line.ends_with(' ');
    let active = if trailing_space {
        String::new()
    } else {
        args.last().cloned().unwrap_or_default()
    };
    let start = line.len() - active.len();
    (
        Request {
            line: line.into(),
            cursor: line.chars().count(),
            context_line: line.into(),
            context_cursor: line.chars().count(),
            replace: start..line.len(),
            cwd: ".".into(),
            generation: 1,
            context_key: String::new(),
        },
        Invocation {
            key: "fixture".into(),
            command: None,
            program: "fixture".into(),
            args,
            active,
            trailing_space,
        },
    )
}

fn options(line: &str) -> Vec<String> {
    let graph = crate::parser::help::parse(
        "Usage: fixture [--json | --plain] [--tag <tag>]... [<file>...]\n\nOptions:\n  --json  JSON output\n  --plain  Plain output\n  --tag <tag>  Add a tag multiple times\n",
        "fixture",
    );
    let (request, invocation) = context(line);
    candidates::complete(
        &graph.root,
        &invocation,
        &request,
        &mut FilesystemEngine::new(),
    )
    .into_iter()
    .map(|item| item.insert)
    .collect()
}

#[test]
fn incompatible_and_consumed_options_are_hidden_but_repeatable_options_remain() {
    let items = options("fixture --json --tag first --");
    assert!(!items.contains(&"--json".into()), "{items:?}");
    assert!(!items.contains(&"--plain".into()), "{items:?}");
    assert!(items.contains(&"--tag".into()), "{items:?}");
    assert!(options("fixture -- --").is_empty());
}

#[test]
fn attached_optional_and_separate_required_values_have_distinct_consumption() {
    let graph = crate::parser::help::parse(
        "Options:\n  --mode[=fast|slow]  Optional mode\n  --output=<file>  Output file\n  --name <name>  Required name\n  --verbose  Verbose output\n",
        "fixture",
    );
    for line in [
        "fixture --mode ",
        "fixture --output ",
        "fixture --name --verbose",
        "fixture --name --ver",
        "fixture -- --mode=fast",
    ] {
        let (request, invocation) = context(line);
        assert!(
            arguments::pending_value(&graph.root, &invocation, &request).is_none(),
            "{line}"
        );
    }
    for (line, query, start) in [
        ("fixture --mode=s", "s", 15),
        ("fixture --name ada", "ada", 15),
        ("fixture --output=", "", 17),
    ] {
        let (request, invocation) = context(line);
        let (_, actual, replace) =
            arguments::pending_value(&graph.root, &invocation, &request).expect("value context");
        assert_eq!(actual, query, "{line}");
        assert_eq!(replace.start, start, "{line}");
    }
}

#[test]
fn repeated_positionals_advance_past_options_and_their_values() {
    let graph = crate::parser::help::parse(
        "Usage: fixture --name <name> <source> <target>...\n",
        "fixture",
    );
    for line in [
        "fixture --name ada input ",
        "fixture --name ada input output ",
    ] {
        let (_, invocation) = context(line);
        assert_eq!(
            arguments::next_positional(&graph.root, &invocation)
                .expect("target")
                .name,
            "target"
        );
    }
}
