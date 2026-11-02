pub(super) struct Page {
    pub(super) usage: &'static str,
    pub(super) commands: &'static [(&'static str, &'static str)],
    pub(super) notes: &'static [&'static str],
}

pub(super) const ROOT: Page = Page {
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

pub(super) const DEV: Page = Page {
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
            "link <path>",
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

pub(super) const DEBUG: Page = Page {
    usage: "Usage: sopra debug <command>",
    commands: &[
        (
            "syntax <text>",
            "Show byte ranges, syntax categories, terminal colors, and escaped token text",
        ),
        (
            "completions",
            "Dump a command's completion graph: [--root|--chunk] <command> [path ...] [--cwd <dir>]",
        ),
    ],
    notes: &[
        r#"Quote shell text to prevent your shell from expanding or executing it: sopra debug syntax 'echo "$HOME" | cat'"#,
        "Examples: sopra debug completions --root npm; sopra debug completions --chunk npm access",
        "Without --root or --chunk, completions performs the full recursive graph inspection and may run help probes.",
    ],
};
