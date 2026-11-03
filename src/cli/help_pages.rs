pub(super) struct Page<'a> {
    pub(super) usage: &'a str,
    pub(super) commands: &'a [(&'a str, &'a str)],
    pub(super) notes: &'a [&'a str],
}

pub(super) const ROOT: Page<'static> = Page {
    usage: "Usage: sopra <command>",
    commands: ROOT_COMMANDS,
    notes: &[
        "Run session commands after submitting the current command.",
        "Configuration: $XDG_CONFIG_HOME/sopra/config.toml (default: ~/.config/sopra/config.toml).",
    ],
};

const ROOT_COMMANDS: &[(&str, &str)] = &[
    (
        "refresh",
        "Reload config and reconcile widgets, retaining their state",
    ),
    ("restart", "Replace the worker and reset widget state"),
    (
        "config edit",
        "Open config.toml in $VISUAL or $EDITOR, validate, then refresh",
    ),
    (
        "config check",
        "Validate config, templates, and widget references",
    ),
    ("config path", "Print the canonical config path"),
    (
        "config show",
        "Print effective configuration, including defaults",
    ),
    (
        "widgets list",
        "List discovered widgets, source paths, and runtime errors",
    ),
    (
        "doctor",
        "Diagnose the worker, Zsh integration, config, and Rust compiler",
    ),
    (
        "dev <command>",
        "Build, check, test, or launch a linked source checkout",
    ),
    (
        "debug <command>",
        "Inspect syntax tokens and completion graphs",
    ),
    ("--version", "Show Sopra's version"),
    ("--help", "Show this help"),
];

pub(super) const DEV: Page<'static> = Page {
    usage: "Usage: sopra dev <command>",
    commands: &[
        ("build", "Build the checkout's debug worker"),
        (
            "check",
            "Check formatting, then run Clippy with warnings as errors",
        ),
        (
            "test [args ...]",
            "Run workspace tests; forward Cargo test filters and options",
        ),
        (
            "shell",
            "Build and launch the checkout's isolated development Zsh",
        ),
        (
            "link <directory>",
            "Save the checkout used by developer commands",
        ),
        ("unlink", "Remove the saved developer checkout link"),
        ("path", "Print the checkout developer commands will use"),
    ],
    notes: &[
        "Checkout selection: saved link, SOPRA_REPO_ROOT, current directory ancestors, then executable ancestors.",
        "Links affect developer commands only. Example: sopra dev test syntax:: -- --nocapture",
    ],
};

pub(super) const DEBUG: Page<'static> = Page {
    usage: "Usage: sopra debug <command>",
    commands: &[
        (
            "candidates [options] <text>",
            "Print completion candidates, insertion text, and replacement ranges as JSON",
        ),
        ("providers", "Validate and list live-value provider recipes"),
        (
            "syntax <text>",
            "Show byte ranges, syntax categories, terminal colors, and escaped token text",
        ),
        (
            "graph <command>",
            "Generate graphs, parse captured sources, and inspect discovery caches",
        ),
        (
            "completions [options] <command> [path ...]",
            "Dump a command's completion graph",
        ),
    ],
    notes: &[
        r#"Quote shell text to prevent your shell from expanding or executing it: sopra debug syntax 'echo "$HOME" | cat'"#,
        "Examples: sopra debug completions --root npm; sopra debug completions --chunk npm access",
        "Without --root or --chunk, completions performs the full recursive graph inspection and may run help probes.",
    ],
};

pub(super) const GRAPH: Page<'static> = Page {
    usage: "Usage: sopra debug graph <command>",
    commands: &[
        (
            "explain [options] <command> [path ...] -- <entry>",
            "Explain a graph entry using source lines and merge observations",
        ),
        (
            "json [options] <command> [path ...]",
            "Print graph fields and argument grammar as JSON",
        ),
        (
            "generate [options] <command> [path ...]",
            "Generate a completion graph from live or cached sources",
        ),
        (
            "sources [options] <command> [path ...]",
            "Print the help and manual evidence used for discovery",
        ),
        (
            "cache [options] <command> [path ...]",
            "Inspect the binary bundle, identity, age, and source sizes",
        ),
        (
            "parse [--man] <command> <file> [path ...]",
            "Parse captured help or manual text without running the command",
        ),
    ],
    notes: &[
        "Use --root for immediate children or --chunk with a nested command path. Generation otherwise inspects the full graph.",
        "Use --fresh to collect new sources. Parsing accepts - as the input file for stdin.",
    ],
};
