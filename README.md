<div align="center">
  <h1>Sopra</h1>
  <p><code>/ˈsoʊ.prə/</code></p>
  <p>An experimental line editor for interactive Zsh</p>
  <img width="700" alt="Sopra showing syntax highlighting and contextual completions" src="./assets/sopra.png">
</div>

**Sopra** provides interactive line editing, syntax highlighting, and contextual completions inside Zsh. Zsh keeps ownership of command execution, shell state, aliases, functions, and environment variables.

Sopra is experimental (`0.1.0`), targets interactive Zsh on Unix-like systems,
and is tested on macOS and Linux. Editing behavior and configuration may change.

## Features

- Unicode-safe editing, history search, autosuggestions, and multiline input.
- Syntax highlighting and contextual completion for commands, paths, options, values, and nested subcommands.
- Configurable prompts, transient command lines, and Rust prompt widgets.
- Optional mouse editing with native terminal scrollback and search.

## Install

Requires Zsh, Bash, Rust and Cargo, `curl`, `tar`, and `install`. The installer
builds locally and needs network access.

```sh
curl -fsSL https://raw.githubusercontent.com/domenicurso/sopra/main/install.sh | bash
```

Enable Sopra in an interactive Zsh:

```sh
source "$HOME/.local/share/sopra/editor.zsh"
```

Add that line to `~/.zshrc` to enable it in future shells. The installer defaults
to `main` and `~/.local`; override these with `SOPRA_REF` and `SOPRA_PREFIX`.
With a custom prefix, source its `share/sopra/editor.zsh`. It installs a `sopra`
launcher under the prefix's `bin` directory and leaves startup files unchanged.
See [building from source](docs/development.md#build-from-source) to run a checkout.

## Get started

```sh
sopra config edit   # Edit, validate, and reload configuration
sopra --help        # Browse commands
sopra doctor       # Diagnose the current setup
```

Configuration lives at `~/.config/sopra/config.toml`, or
`$XDG_CONFIG_HOME/sopra/config.toml`. Enter submits a command; Shift+Enter adds a
line. Ctrl+R searches history, and Ctrl+G toggles mouse editing.

## Documentation

- [Editing and commands](docs/usage.md): key bindings, history, session controls, and cache commands.
- [Configuration and prompts](docs/configuration.md): options, templates, colors, and built-in widgets.
- [Rust prompt widgets](docs/widgets.md): write custom widgets.
- [Development](docs/development.md): source builds, linked checkouts, debugging, and contributing.
- [Runtime ownership](docs/architecture.md) and [completion architecture](docs/completions.md): how the system works.
- [Terminal testing](docs/terminal-sessions.md): persistent interactive test sessions.

## License

[GNU General Public License v3.0](LICENSE).
