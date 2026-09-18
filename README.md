# Sopra

Sopra is a fast, Rust-owned line editor for your existing Zsh session. It gives the prompt a responsive editor, syntax-aware rendering, and useful completions while Zsh continues to own command execution, history, keymaps, and the shell lifecycle.

## What you get

- A responsive editor with Unicode-safe cursor movement, word editing, history navigation, kill/yank operations, and transient command lines.
- Live syntax highlighting for commands, strings, variables, numbers, delimiters, and incomplete expressions.
- Completion for commands, aliases, functions, variables, arrays, paths, options, values, and recursively discovered CLI subcommands.
- Descriptions assembled from command help, subcommand help, completion scripts, and manual pages, with results cached for later prompts.
- A terminal-native overlay that redraws cleanly across resize, Escape, Ctrl-C, and accepted commands.

Sopra is a line editor, not a replacement shell. Your commands still run in the Zsh you already configured, so existing aliases, functions, plugins, history, and environment remain available.

## Try it from a checkout

You need Rust with Cargo and an installed Zsh.

```sh
./scripts/start.sh
```

The launcher builds the current checkout, starts an isolated interactive Zsh, and loads [`zsh/editor.zsh`](zsh/editor.zsh). It does not modify, rebuild, or replace your normal Zsh installation.

## Install

The installer downloads the requested GitHub revision, builds the `sopra` binary, and installs the Zsh bridge. It does not edit `.zshrc` automatically.

```sh
curl -fsSL https://raw.githubusercontent.com/domenicurso/sopra/main/install.sh | bash
```

By default, the binary goes to `~/.local/bin/sopra` and the bridge goes to `~/.local/share/sopra/editor.zsh`. Add the binary directory to `PATH`, then add this line to your interactive Zsh configuration:

```sh
source "$HOME/.local/share/sopra/editor.zsh"
```

The installer accepts these environment variables when you need a different source or destination:

```sh
SOPRA_REPOSITORY=owner/repository
SOPRA_REF=branch-or-tag
SOPRA_PREFIX="$HOME/.local"
```

## Shell configuration

The bridge reads the current prompt, right prompt, command path, aliases, functions, variables, arrays, and Zsh completion path from the running shell. You can override the editor settings with:

```sh
export SOPRA_BIN="$HOME/.local/bin/sopra"
export SOPRA_PROMPT='%n in %~ $ '
export SOPRA_RPROMPT=''
export SOPRA_TRANSIENT_PROMPT='$ '
```

Sopra also accepts direct editor options, which are useful for integrations and tests:

```text
sopra [--buffer TEXT] [--cursor N] [--prompt TEXT] [--cwd PATH]
```

## Key behavior

- Enter accepts the current command and returns it to Zsh.
- Tab opens completions and accepts the selected completion.
- Escape hides the completion overlay while keeping the prompt active.
- Ctrl-C preserves the typed line as a transient line and starts a fresh prompt.
- Arrow keys delegate history movement to Zsh.

## Development

Run the focused checks from the repository root:

```sh
./scripts/build.sh
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
python3 scripts/terminal-harness.py
for file in zsh/*.zsh; do zsh -n "$file"; done
bash -n install.sh scripts/*.sh
```

The PTY harness exercises the real shell bridge, rendering, palette queries, command and path completion, transient lines, resizing, Escape, Ctrl-C, and command execution. The Rust tests cover the editor, parser, completion providers, ranking, syntax spans, and rendering primitives.

## License

Sopra is licensed under MIT OR Apache-2.0.
