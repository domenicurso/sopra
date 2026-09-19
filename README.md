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

The installer builds the ref named by `SOPRA_REF` (which defaults to `main`) and installs the binary and Zsh bridge under `~/.local`. It does not edit shell startup files.

From an interactive Zsh, enable Sopra in the current shell with:

```sh
export PATH="$HOME/.local/bin:$PATH"
source "$HOME/.local/share/sopra/editor.zsh"
```

Add the `source` line to `~/.zshrc` if Sopra should load in future interactive shells. If you install with a custom `SOPRA_PREFIX`, source `share/sopra/editor.zsh` from that prefix instead. The one-line installer is a convenience path; use the source-build instructions below when you need to inspect or pin the checkout first.

### Install from source

```sh
git clone https://github.com/domenicurso/sopra.git
cd sopra
cargo build --release --locked --bin sopra
export SOPRA_BIN="$PWD/target/release/sopra"
source "$PWD/zsh/editor.zsh"
```

Keep `SOPRA_BIN` and the `source` command in your Zsh configuration if you want to run this checkout directly.

## Features

- Unicode-safe editing with history, word movement, kill/yank operations, and transient command lines.
- Live highlighting for commands, strings, variables, numbers, delimiters, and incomplete expressions.
- Contextual completion for shell commands, aliases, functions, variables, arrays, paths, options, values, and recursive CLI subcommands.

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
| `SOPRA_BIN` | Auto-detected | Path to the Sopra executable. |
| `SOPRA_PROMPT` | Current `PROMPT` | Prompt rendered while editing. |
| `SOPRA_RPROMPT` | Current `RPROMPT` | Right prompt rendered while editing. |
| `SOPRA_TRANSIENT_PROMPT` | `$ ` | Prompt shown after accepting or interrupting a line. |

## How it works

Sopra runs as the editor behind Zsh's ZLE integration rather than as a replacement shell. The Zsh bridge passes the current buffer, cursor, prompt, working directory, and completion context to the Rust binary; Sopra returns the edited buffer and the action Zsh should perform. Zsh then accepts the command and executes it using the existing shell environment.

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

## Contributing

Keep changes focused on editor, Zsh integration, or completion behavior, and add a focused test for behavior changes. Run the full check sequence above before opening a pull request. Bug reports are most useful when they include the operating system, Zsh version, terminal, installation method, and a minimal reproduction.

## License

Sopra is licensed under the [GNU General Public License v3.0](./LICENSE).
