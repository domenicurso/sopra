use super::super::parse;

#[test]
fn slash_separated_option_references_remain_description_text() {
    let graph = parse(
        "OPTIONS:\n    -C NUM, --context=NUM\n        Show lines around each match.\n        -B/--before-context, -A/--after-context, -C/--context or --passthru\n        override this flag.\n    --passthru\n        Print all lines.\n",
        "tool",
    );
    assert_eq!(graph.root.options.len(), 2);
    assert!(
        graph
            .root
            .options
            .iter()
            .all(|option| { option.names.iter().all(|name| !name.contains('/')) })
    );
    assert!(
        graph.root.options[0]
            .description
            .as_deref()
            .is_some_and(|text| {
                text.contains("-B/--before-context") && text.contains("override this flag")
            })
    );
}

#[test]
fn colon_suffixed_commands_and_uppercase_usage_are_structural() {
    let graph = parse(
        "Work with repositories\n\nUSAGE\n  gh <command> [flags]\n\nCORE COMMANDS\n  repo:  Manage repositories\n  pr:    Manage pull requests\n\nHELP TOPICS\n  environment: Learn about environment variables\n\nFLAGS\n  --help  Show help\n",
        "gh",
    );
    assert_eq!(
        graph
            .root
            .subcommands
            .iter()
            .map(|node| node.name.as_str())
            .collect::<Vec<_>>(),
        ["repo", "pr"]
    );
    assert_eq!(
        graph.root.subcommands[0].description.as_deref(),
        Some("Manage repositories")
    );
    assert!(graph.root.synopsis.is_some());
}

#[test]
fn overflowing_labels_and_environment_tables_are_not_description_continuations() {
    let graph = parse(
        "Usage: deno [OPTIONS] [COMMAND]\n\nCommands:\n  Execution:\n    run          Run a script\n    bump-version Update the version\n    approve-scripts Approve scripts\n\nEnvironment variables:\n  DENO_DIR       Set the cache directory\n  HTTP_PROXY     Proxy address\n",
        "deno",
    );
    assert_eq!(
        graph
            .root
            .subcommands
            .iter()
            .map(|node| node.name.as_str())
            .collect::<Vec<_>>(),
        ["run", "bump-version", "approve-scripts"]
    );
    assert!(
        graph
            .root
            .subcommands
            .iter()
            .all(|node| node.subcommands.is_empty())
    );
    assert_eq!(
        graph.root.subcommands[1].description.as_deref(),
        Some("Update the version")
    );
}

#[test]
fn nested_manual_headings_and_value_tables_do_not_create_commands() {
    let graph = parse(
        "curl(1) curl Manual curl(1)\n\nNAME\n     curl - transfer a URL\n\nSYNOPSIS\n     curl [options / URLs]\n\nOPTIONS\n     --quote <command>\n          Send commands before a transfer.\n          commands:\n            rename  Rename a remote file\n            pwd     Show the remote directory\n     --output <file>\n          Write output to a file\n",
        "curl",
    );
    assert!(graph.root.subcommands.is_empty());
    assert!(graph.root.description.is_none());
    assert!(
        graph
            .root
            .options
            .iter()
            .any(|option| option.names == ["--output"])
    );
}

#[test]
fn tabs_and_spaces_share_the_same_table_geometry() {
    let graph = parse(
        "Commands:\n\tbuild\tCompile the project\n\ttest\tRun the tests\n",
        "tool",
    );
    assert_eq!(graph.root.subcommands.len(), 2);
    assert_eq!(
        graph.root.subcommands[0].description.as_deref(),
        Some("Compile the project")
    );
}

#[test]
fn qualified_manual_headings_end_command_tables() {
    let graph = parse(
        "GIT COMMANDS\n  add  Add content\nHIGH-LEVEL COMMANDS (PORCELAIN)\n     o  A paragraph bullet\nLOW-LEVEL COMMANDS (PLUMBING)\n     Although Git includes commands, this is prose.\n",
        "git",
    );
    assert_eq!(graph.root.subcommands.len(), 1);
    assert_eq!(graph.root.subcommands[0].name, "add");
}

#[test]
fn repeatable_bare_arguments_do_not_become_literal_subcommands() {
    let graph = parse(
        "Usage: bun pm [<command>]\n\nCommands:\n  bun pm trust names ...  Trust named packages\n",
        "bun",
    );
    let trust = graph
        .root
        .find(&["pm".into(), "trust".into()])
        .expect("trust command");
    assert!(trust.subcommands.is_empty());
    assert_eq!(trust.positionals[0].name, "names");
    assert!(trust.positionals[0].repeatable);
}
