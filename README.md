# Keel

Keel is a prototype Rust-owned line editor for stock Zsh. It recreates the current interactive surface for demonstration purposes: a command line, a selectable overlay, and a visibly animated cursor, while Zsh keeps ownership of prompts, command parsing, execution, history, and scrollback.

The prototype deliberately has no real completion provider. Its overlay is static demo data, so the architecture can be exercised without pretending that the shell integration is production-ready.

## Run the demo

Requirements are a Rust toolchain and an installed `zsh`.

```sh
./scripts/start-keel.sh
```

The launcher builds `keel-demo`, starts stock Zsh with an isolated `ZDOTDIR`, and sources the small bridge in [`zsh/keel.zsh`](zsh/keel.zsh). It does not patch, rebuild, or replace the user's Zsh.

Inside the demo:

- Type to filter the static overlay.
- Use Up/Down to change the selection and Tab to insert it.
- Use Ctrl-L to hide or show the overlay.
- Press Enter to hand the line back to Zsh for normal execution.
- Press Escape to leave the Rust editor and keep the edited line in stock ZLE.
- Press Ctrl-C to abort the line and let Zsh start a fresh prompt beneath it.

## Architecture

The ownership boundary is intentionally small:

```text
stock Zsh ZLE
    │  zle-line-init invokes one tiny widget
    ▼
Rust editor process
    │  owns input, state, scene composition, and the 60 Hz loop
    ▼
one ratatui Buffer
    │  prompt, command text, overlay, status, and cursor are Elements
    ▼
one diffed ANSI renderer
    │  writes directly to /dev/tty
    ▼
terminal
```

The Rust process reads the current `BUFFER` and character-based `CURSOR` from ZLE, opens the existing terminal directly, enables raw mode for the duration of the editor, and writes a tiny result record back through temporary files. On accept, the scene briefly replaces the full prompt with a compact transient `❯` prompt before the widget restores the edited buffer and calls Zsh's built-in `.accept-line`; Zsh then executes the command itself. Ctrl-C uses the same transient scene, clears only the internal ZLE buffer, and accepts that empty line so the painted command remains visible without executing. Escape returns the edited buffer to ordinary ZLE redisplay. The app does not create a PTY, pass-through renderer, native shim, patched Zsh, or completion ABI.

Every visible piece implements the same `Element` trait and paints into the same frame. The cursor is therefore an ordinary scene element whose color changes at 60 Hz, while the overlay and future surfaces can use the same buffer, diff, and ANSI path without adding a second renderer.

The editor is a separate process because stock Zsh does not expose a public API for replacing its entire ZLE input loop from Rust. That process boundary is a small file protocol, not a binary ABI, and it lets the prototype work with the Zsh already installed on the user's machine.

## Development

```sh
cargo fmt --all -- --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
python3 scripts/terminal-harness.py
zsh -n zsh/keel.zsh
bash -n scripts/*.sh
```

The terminal harness allocates a real pseudo-terminal and drives the shell in both directions, so it catches redraw-position and shell-return bugs that a one-way output capture misses.

The repository is intentionally small:

- [`src/editor.rs`](src/editor.rs) coordinates the editor loop, while its child modules own editing and view state.
- [`src/scene.rs`](src/scene.rs) defines the homogeneous scene contract; child modules own the canvas, elements, overlay, and scene layouts.
- [`src/render.rs`](src/render.rs) diffs ratatui cells and writes ANSI updates, with ANSI encoding kept beside it.
- [`src/input.rs`](src/input.rs) owns direct-TTY input, raw mode, and terminal size, with key decoding kept beside it.
- [`zsh/keel.zsh`](zsh/keel.zsh) is the only shell-side bridge.
- [`demo/.zshrc`](demo/.zshrc) provides the isolated demo startup file.

This branch is a rendering and ownership proof. Real completions, shell-wide history integration, richer ZLE editing behavior, resize polish, and installation are intentionally deferred until the boundary proves stable.

## License

Keel is licensed under MIT OR Apache-2.0.
