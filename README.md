<div align="center">
  
  <h1>Sopra</h1>
  <p><code>/ˈsoʊ.prə/</code></p>
  <p>A Rust-owned Zsh line editor for the future</p>
  <img width="700" alt="Sopra" src="./assets/sopra.png" />
</div>

**Sopra** adds fast editing, syntax highlighting, and contextual completions while Zsh continues to own command execution and the shell environment.

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/domenicurso/sopra/main/install.sh | bash
```

The installer builds the current Sopra source and installs the binary and Zsh bridge under `~/.local`. It leaves shell startup configuration unchanged while the prompt integration is being formalized.

## Features

- Unicode-safe editing with history, word movement, kill/yank operations, and transient command lines.
- Live highlighting for commands, strings, variables, numbers, delimiters, and incomplete expressions.
- Contextual completion for shell commands, aliases, functions, variables, arrays, paths, options, values, and recursive CLI subcommands.
- CLI descriptions collected from help output, completion scripts, and manual pages, then cached for later prompts.

## Controls

- `Enter` accepts the command.
- `Tab` opens or accepts completion.
- `Escape` hides the completion menu.
- `Ctrl-C` preserves the current line and starts a fresh prompt.

## Development

The checkout launcher is for development and testing:

```sh
git clone https://github.com/domenicurso/sopra.git
cd sopra
./scripts/start.sh
```

Run the checks from the repository root:

```sh
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
python3 scripts/terminal-harness.py
```

## License

Sopra is licensed under MIT OR Apache-2.0.
