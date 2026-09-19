use super::super::{parse, parse_for};

#[test]
fn ignores_prose_after_a_man_section_changes() {
    let graph = parse_for(
        "NAME:\ngit-diff - Show changes\nSYNOPSIS:\ngit diff [<options>]\nDESCRIPTION:\nShow changes between commits.\nExample:\n-static void describe(struct commit *cmit, int last_one)\nOPTIONS:\n-p, --patch  Generate patch\nSEE ALSO:\ngit-diff-index(1) compares trees\n",
        "git",
        &["diff".to_string()],
    );
    assert!(graph.root.subcommands.is_empty());
    assert!(
        graph
            .root
            .options
            .iter()
            .any(|option| option.names == ["-p", "--patch"])
    );
    assert!(
        !graph
            .root
            .options
            .iter()
            .any(|option| option.names == ["-static"])
    );
}

#[test]
fn ignores_category_sentences_in_command_tables() {
    let graph = parse(
        "Commands:\nstart a working area\n  clone  Clone a repository\n  init   Initialize a repository\n",
        "git",
    );
    assert!(
        !graph
            .root
            .subcommands
            .iter()
            .any(|command| command.name == "start")
    );
    assert!(
        graph
            .root
            .subcommands
            .iter()
            .any(|command| command.name == "clone")
    );
}

#[test]
fn ignores_colon_terminated_command_categories() {
    let graph = parse(
        "Commands:\n  Execution:\n    run  Run a program\n  Tooling:\n    check  Check a project\n",
        "deno",
    );
    assert_eq!(
        graph
            .root
            .subcommands
            .iter()
            .map(|command| command.name.as_str())
            .collect::<Vec<_>>(),
        ["run", "check"]
    );
}

#[test]
fn keeps_command_examples_out_of_descriptions() {
    let graph = parse(
        "Commands:\n  add       react                 Add a dependency to package.json\n  link      [<package>]          Register or link a local package\n",
        "bun",
    );
    let add = graph
        .root
        .subcommands
        .iter()
        .find(|command| command.name == "add")
        .expect("add command");
    assert_eq!(
        add.description.as_deref(),
        Some("Add a dependency to package.json")
    );
    let link = graph
        .root
        .subcommands
        .iter()
        .find(|command| command.name == "link")
        .expect("link command");
    assert_eq!(link.positionals[0].name, "package");
}

#[test]
fn preserves_embedded_optional_syntax_in_argument_names() {
    let graph = parse(
        "Commands:\n  bun view name[@version]  view package metadata\n",
        "bun",
    );
    let view = graph
        .root
        .subcommands
        .iter()
        .find(|command| command.name == "view")
        .expect("view command");
    assert_eq!(view.positionals[0].name, "name[@version]");
}

#[test]
fn usage_lines_build_nested_subcommands_for_recursive_help() {
    let graph = parse_for(
        "Usage:\nnpm access list packages [<package>]\nnpm access collaborators [<package>]\nnpm access get status [<package>]\n\nOptions:\n  --json  output JSON\n  --install-strategy <hoisted|nested>  dependency tree strategy\n",
        "npm",
        &["access".to_string()],
    );
    let names = graph
        .root
        .subcommands
        .iter()
        .map(|command| command.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, ["list", "collaborators", "get"]);
    assert!(
        graph
            .root
            .options
            .iter()
            .any(|option| option.names == ["--json"])
    );
    let strategy = graph
        .root
        .options
        .iter()
        .find(|option| option.names == ["--install-strategy"])
        .expect("nested option");
    assert_eq!(
        strategy.value.as_ref().map(|value| value.name.as_str()),
        Some("hoisted|nested")
    );
}

#[test]
fn keeps_nested_usage_arguments_out_of_the_parent_node() {
    let graph = parse_for(
        "Usage:\nnpm access list packages [<user>] [<package>]\nnpm access get status [<package>]\nnpm access set status=public|private [<package>]\nnpm access grant <read-only|read-write> <scope:team> [<package>]\n",
        "npm",
        &["access".to_string()],
    );
    assert!(graph.root.positionals.is_empty());
    assert!(graph.root.subcommands.iter().any(|command| {
        command.name == "list"
            && command.positionals[0].value.choices == ["packages"]
            && command.positionals[1].name == "user"
    }));
    assert!(graph.root.subcommands.iter().any(|command| {
        command.name == "set"
            && command.positionals[0]
                .value
                .choices
                .contains(&"status=public".to_string())
    }));
}

#[test]
fn treats_parenthetical_usage_notes_as_prose() {
    let graph = parse_for(
        "Usage:\ncli get [<key> ...] (See `cli config`)\ncli init <package-spec (with version)> (same as `npx create-package`)\n",
        "cli",
        &["get".to_string()],
    );
    let names = graph
        .root
        .positionals
        .iter()
        .map(|positional| positional.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, ["key"]);
    assert!(
        !names
            .iter()
            .any(|name| matches!(*name, "See" | "same" | "with"))
    );
}

#[test]
fn ignores_or_prefixes_in_multiform_usage() {
    let graph = parse_for(
        "usage: git diff [<options>] [<path>...]\n   or: git diff --cached [<path>...]\n\nOptions:\n  --cached  compare staged changes\n",
        "git",
        &["diff".to_string()],
    );
    assert!(
        !graph
            .root
            .subcommands
            .iter()
            .any(|command| command.name == "or:")
    );
    assert!(
        graph
            .root
            .options
            .iter()
            .any(|option| option.names == ["--cached"])
    );
}

#[test]
fn does_not_promote_exact_command_descriptions_to_children() {
    let graph = parse_for(
        "Commands:\n  opencode providers           manage AI providers and credentials [aliases: auth]\n  opencode providers list      list providers [aliases: ls]\n",
        "opencode",
        &["providers".to_string()],
    );
    assert_eq!(
        graph
            .root
            .subcommands
            .iter()
            .map(|command| command.name.as_str())
            .collect::<Vec<_>>(),
        ["list"]
    );
}

#[test]
fn preserves_nested_help_tree_rows() {
    let graph = parse_for(
        "Commands:\n  bun pm pack                 create a tarball\n  ├ --dry-run                 skip writing the tarball\n  └ --filename                choose the output name\n  bun pm pkg                  manage package data\n  ├ get [key ...]\n  └ set key=value ...\n",
        "bun",
        &["pm".to_string()],
    );
    let pack = graph
        .root
        .subcommands
        .iter()
        .find(|command| command.name == "pack")
        .expect("pack command");
    assert!(pack.options.iter().any(|option| {
        option.names == ["--dry-run"]
            && option.description.as_deref() == Some("skip writing the tarball")
    }));
    let pkg = graph
        .root
        .subcommands
        .iter()
        .find(|command| command.name == "pkg")
        .expect("pkg command");
    let get = pkg
        .subcommands
        .iter()
        .find(|command| command.name == "get")
        .expect("get command");
    assert_eq!(get.positionals[0].name, "key");
    assert!(get.positionals[0].repeatable);
}

#[test]
fn enriches_existing_option_values_from_possible_values_metadata() {
    let graph = parse(
        "Options:\n  --edition <YEAR>  Edition to set for the generated crate [possible values: 2015, 2018, 2021, 2024]\n",
        "cargo",
    );
    let edition = graph
        .root
        .options
        .iter()
        .find(|option| option.names == ["--edition"])
        .expect("edition option");
    assert_eq!(
        edition.value.as_ref().expect("edition value").choices,
        ["2015", "2018", "2021", "2024"]
    );
}

#[test]
fn combines_repeated_attached_values_into_choices() {
    let graph = parse(
        "Options:\n  --mode=att  emit AT&T syntax\n  --mode=intel  emit Intel syntax\n",
        "objdump",
    );
    let mode = graph
        .root
        .options
        .iter()
        .find(|option| option.names == ["--mode"])
        .expect("mode option");
    assert_eq!(
        mode.value.as_ref().expect("mode value").choices,
        ["att", "intel"]
    );
}

#[test]
fn parses_option_tables_under_command_sections() {
    let graph = parse(
        "COMMANDS\n  -a, --archive  archive the input\n  -d, --debug\n          show debug details\n",
        "objdump",
    );
    assert!(graph.root.subcommands.is_empty());
    assert!(graph.root.options.iter().any(|option| {
        option.names == ["-a", "--archive"]
            && option.description.as_deref() == Some("archive the input")
    }));
    assert!(graph.root.options.iter().any(|option| {
        option.names == ["-d", "--debug"]
            && option.description.as_deref() == Some("show debug details")
    }));
}

#[test]
fn extracts_choices_from_prose_value_lists() {
    let graph = parse(
        "Options:\n  --color=<mode>  Valid options are \"on\", \"off\" and \"terminal\" (default)\n  --style=<kind>  Options: addrs (default), names, both (requires a file)\n",
        "tool",
    );
    let color = graph
        .root
        .options
        .iter()
        .find(|option| option.names == ["--color"])
        .expect("color option");
    assert_eq!(
        color.value.as_ref().expect("color value").choices,
        ["on", "off", "terminal"]
    );
    let style = graph
        .root
        .options
        .iter()
        .find(|option| option.names == ["--style"])
        .expect("style option");
    assert_eq!(
        style.value.as_ref().expect("style value").choices,
        ["addrs", "names", "both"]
    );
}

#[test]
fn keeps_the_richest_description_when_sources_repeat_an_option() {
    let graph = parse(
        "Options:\n  --agent  -\n  --agent  select the agent used for this session\n",
        "tool",
    );
    let agent = graph
        .root
        .options
        .iter()
        .find(|option| option.names == ["--agent"])
        .expect("agent option");
    assert_eq!(
        agent.description.as_deref(),
        Some("select the agent used for this session")
    );
}

#[test]
fn stops_option_descriptions_before_a_command_table() {
    let graph = parse(
        "Options:\n  -h, --help  Print help\n\nCommands:\n    build, b    Compile the current package\n    check       Analyze the current package\n",
        "cargo",
    );
    let help = graph
        .root
        .options
        .iter()
        .find(|option| option.names == ["-h", "--help"])
        .expect("help option");
    assert_eq!(help.description.as_deref(), Some("Print help"));
    let build = graph
        .root
        .subcommands
        .iter()
        .find(|command| command.name == "build")
        .expect("build command");
    assert_eq!(build.aliases, ["b"]);
}
