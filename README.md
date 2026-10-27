<div align="center">
  <h1>Sopra</h1>
  <p><code>/ˈsoʊ.prə/</code></p>
  <p>An experimental line editor for interactive Zsh</p>
  <img width="700" alt="Sopra showing syntax highlighting and contextual completions" src="./assets/sopra.png">
</div>

**Sopra** provides interactive line editing, syntax highlighting, and contextual completions inside Zsh. Zsh keeps ownership of command execution, shell state, aliases, functions, and environment variables.

## Status

Sopra is early-stage software at version `0.1.0`. It currently targets interactive Zsh on Unix-like systems; continuous integration covers macOS and Linux. Editing behavior and configuration may change while the integration settles.

## Install

### Requirements

The installer needs Zsh, Bash, Rust and Cargo, `curl`, `tar`, and the `install` command. It builds Sopra locally, so the machine needs a working Rust toolchain and network access to download the source archive.

### Quick install

```sh
curl -fsSL https://raw.githubusercontent.com/domenicurso/sopra/main/install.sh | bash
```

The installer builds the ref named by `SOPRA_REF` (which defaults to `main`) and installs the private worker and Zsh bridge under `~/.local`. It does not edit shell startup files or add a public executable to `PATH`.

From an interactive Zsh, enable Sopra in the current shell with:

```sh
source "$HOME/.local/share/sopra/editor.zsh"
```

Add the `source` line to `~/.zshrc` if Sopra should load in future interactive shells. If you install with a custom `SOPRA_PREFIX`, source `share/sopra/editor.zsh` from that prefix instead. The one-line installer is a convenience path; use the source-build instructions below when you need to inspect or pin the checkout first.

### Install from source

```sh
git clone https://github.com/domenicurso/sopra.git
cd sopra
cargo build --release --locked --bin sopra-worker
export SOPRA_BIN="$PWD/target/release/sopra-worker"
source "$PWD/zsh/editor.zsh"
```

Keep `SOPRA_BIN` and the `source` command in your Zsh configuration if you want to run this checkout directly.

## Features

- Unicode-safe editing with history, word movement, kill/yank operations, and transient command lines.
- Live highlighting for commands, strings, variables, numbers, delimiters, and incomplete expressions.
- Contextual completion for shell commands, aliases, functions, variables, arrays, paths, options, values, and recursive CLI subcommands.
- `sopra settings` opens an interactive settings modal when the Sopra Zsh integration is loaded.

## Configuration

The installer accepts these environment variables:

| Variable | Default | Purpose |
| --- | --- | --- |
| `SOPRA_REPOSITORY` | `domenicurso/sopra` | GitHub repository to download. |
| `SOPRA_REF` | `main` | Branch, tag, or commit archive to build. |
| `SOPRA_PREFIX` | `$HOME/.local` | Installation prefix for the binary and Zsh bridge. |

The Zsh bridge accepts these runtime variables:

| Variable | Default | Purpose |
| --- | --- | --- |
| `SOPRA_BIN` | Auto-detected | Development override for the private Sopra worker. |
| `SOPRA_PROMPT` | Colored `{path}` and Zsh prompt character | Prompt rendered while editing. |
| `SOPRA_RPROMPT` | Colored `{git}` | Right prompt rendered while editing. |
| `SOPRA_TRANSIENT_PROMPT` | `$ ` | Prompt shown after accepting or interrupting a line. |
| `SOPRA_CLOCK_FORMAT` | `%H:%M:%S` | Local time format for `{clock}`. |
| `SOPRA_CURSOR_ANIMATION` | `1` | Enables the animated editing cursor. |
| `SOPRA_SYNTAX_HIGHLIGHTING` | `1` | Enables live syntax highlighting. |
| `SOPRA_COMPLETION_ENABLED` | `1` | Enables contextual completions. |

`sopra settings` edits these values in a centered terminal modal, grouped into Prompts, Editor, and Completion tabs, and saves them to `~/.config/sopra/settings`. Explicit `SOPRA_*` values already present in the shell take precedence when the integration starts.

Prompt templates accept Zsh prompt escapes such as `%F{cyan}`, `%f`, `%~`, and `%#`, plus these widgets:

| Widget | Display |
| --- | --- |
| `{path}` | Current directory, abbreviated relative to your home directory. |
| `{git}` | Current Git branch or detached commit; empty outside a repository. |
| `{clock}` | Local time in `HH:MM:SS`, refreshed while editing. |

Use a widget in either prompt, for example `SOPRA_RPROMPT='%F{magenta}{git}%f  {clock}'`. With `{path}` visible, press Ctrl-G to focus it, Tab to cycle through parent directories, and Enter to change directory while preserving the command being edited. Escape cancels selection. Explicitly set `SOPRA_PROMPT` and `SOPRA_RPROMPT` to your existing Zsh prompt strings if you prefer them to the built-in defaults.

Style text inline with `[dim]...[/]`, `[bold]...[/]`, `[italic]...[/]`, `[underline]...[/]`, or a color tag such as `[cyan]...[/]`. Tags can be nested; `[[` produces a literal `[`. Use `[if git]...[/]` to show a section only when its widget has content. For example:

```zsh
SOPRA_RPROMPT='[if git]{git}[dim] : [/][/]{clock}'
SOPRA_CLOCK_FORMAT='%H[dim]:[/]%M'
```

The separator before the clock disappears outside a Git repository. Clock formats use the system's `strftime` directives, and the clock updates while you edit. Set these variables before sourcing Sopra, or edit the prompt and clock format in `sopra settings`.

## How it works

Sopra runs as the editor behind Zsh's ZLE integration rather than as a replacement shell. The Zsh bridge passes the current buffer, cursor, prompt, working directory, and completion context to the private Rust worker; Sopra returns the edited buffer and the action Zsh should perform. The bridge also defines the shell-native `sopra` function, whose `settings` subcommand opens the worker's settings mode. Zsh then accepts the command and executes it using the existing shell environment.

This keeps shell behavior in Zsh: aliases, functions, variables, command lookup, and command execution continue to use the current interactive shell.

The completion parser's ownership, source flow, and command-agnostic design rules are documented in [docs/completions.md](docs/completions.md).

## Development

The checkout launcher starts a development Zsh session:

```sh
git clone https://github.com/domenicurso/sopra.git
cd sopra
./scripts/start.sh
```

Run the same checks used by CI from the repository root:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
for file in zsh/*.zsh; do zsh -n "$file"; done
bash -n scripts/*.sh
python3 scripts/terminal-harness.py
```

For multi-turn terminal behavior, use the separate persistent PTY and virtual-screen tool described in [docs/terminal-sessions.md](docs/terminal-sessions.md). It lets an agent send named keystrokes, read rendered screen snapshots, resize the session, and preserve shell state across turns without changing the CI harness.

## Contributing

Keep changes focused on editor, Zsh integration, or completion behavior, and add a focused test for behavior changes. Run the full check sequence above before opening a pull request. Bug reports are most useful when they include the operating system, Zsh version, terminal, installation method, and a minimal reproduction.

## License

Sopra is licensed under the [GNU General Public License v3.0](./LICENSE).
