use super::output::{self, Output, wrap};
use std::io;

const COMMANDS: &[(&str, &str)] = &[
    (
        "shell",
        "Start a hosted Zsh session with ordinary wheel scrollback",
    ),
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
    ("--version", "Show Sopra's version"),
    ("--help", "Show this help"),
];

pub(super) fn print() -> io::Result<()> {
    output::print(&render(&Output::detect()))
}

fn render(output: &Output) -> String {
    let mut result = String::new();
    for line in wrap("Usage: sopra <command>", output.width.saturating_sub(1)) {
        result.push_str(&format!("{}\n", output.style(&line, "1")));
    }
    result.push('\n');
    result.push_str(&format!("{}\n", output.style("Commands", "1")));
    for (name, description) in COMMANDS {
        if output.width >= 64 {
            let indent = 18;
            let lines = wrap(description, output.width.saturating_sub(indent + 1));
            for (index, line) in lines.iter().enumerate() {
                let label = if index == 0 { *name } else { "" };
                result.push_str(&format!(
                    "  {}{}{}\n",
                    output.style(label, "1;36"),
                    " ".repeat(indent - 2 - label.len()),
                    output.style(line, "2")
                ));
            }
        } else {
            for line in wrap(name, output.width.saturating_sub(3)) {
                result.push_str(&format!("  {}\n", output.style(&line, "1;36")));
            }
            for line in wrap(description, output.width.saturating_sub(5)) {
                result.push_str(&format!("    {}\n", output.style(&line, "2")));
            }
            result.push('\n');
        }
    }
    result.push('\n');
    for note in [
        "Run session commands after submitting the current command.",
        "Configuration: $XDG_CONFIG_HOME/sopra/config.toml (default: ~/.config/sopra/config.toml).",
    ] {
        for line in wrap(note, output.width.saturating_sub(1)) {
            result.push_str(&output.style(&line, "2"));
            result.push('\n');
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn help_wraps_without_losing_commands_or_styling_redirected_output() {
        for width in [20, 40, 63, 64, 80, 120] {
            let help = render(&Output {
                width,
                color: false,
                terminal: true,
            });
            assert!(
                help.lines().all(|line| line.len() <= width),
                "{width}: {help}"
            );
            for (name, _) in COMMANDS {
                assert!(help.contains(name));
            }
            assert!(!help.contains('\x1b'));
        }
        let help = render(&Output {
            width: 80,
            color: true,
            terminal: true,
        });
        assert!(help.contains("\x1b[1;36mrefresh"));
        assert!(help.contains("\x1b[2mReload config"));
    }
}
