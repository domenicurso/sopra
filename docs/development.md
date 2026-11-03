# Development

## Build from source

```sh
git clone https://github.com/domenicurso/sopra.git
cd sopra
cargo build --release --locked --bin sopra-worker
export SOPRA_BIN="$PWD/target/release/sopra-worker"
source "$PWD/zsh/editor.zsh"
```

Keep `SOPRA_BIN` and the `source` command in your Zsh configuration if you want to run this checkout directly.

## Developer commands

Developer commands use a discovered checkout or a saved link:

```sh
sopra dev link /path/to/sopra
sopra dev check
sopra dev test syntax:: -- --nocapture
sopra dev shell
```

The link is saved beside config.toml in `dev-checkout` and affects developer commands only. Checkout selection prefers the link, then `SOPRA_REPO_ROOT`, then ancestors of the current directory and executable. `build` builds the debug worker; `check` runs formatting and Clippy; `test` runs workspace tests and forwards Cargo arguments; `shell` builds and starts an isolated Zsh. Use `sopra dev path` to inspect the target and `sopra dev unlink` to remove the link. For the first build, use `./scripts/start.sh`.

Inspect syntax without executing it, or inspect discovered completion graphs:

```sh
sopra debug syntax 'echo "$HOME" | cat'
sopra debug completions --root npm
sopra debug completions --chunk npm access
```

Run shell syntax checks and `python3 scripts/terminal-harness.py` alongside `sopra dev check` and `sopra dev test` before opening a pull request. See [terminal sessions](terminal-sessions.md) for persistent interactive testing and `sopra dev --help` / `sopra debug --help` for command details.

## Contributing

Keep changes focused on editor, Zsh integration, or completion behavior, and add a focused test for behavior changes. Run the full check sequence above before opening a pull request. Bug reports are most useful when they include the operating system, Zsh version, terminal, installation method, and a minimal reproduction.

See [runtime ownership](architecture.md) and [completion architecture](completions.md) for subsystem boundaries.
