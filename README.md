# Keel

Keel is a Rust-owned line editor that runs with the user's existing Zsh. Rust owns input, editing state, completion ranking, scene composition, rendering, and cursor animation; Zsh keeps command execution, history, keymap configuration, and the normal shell lifecycle.

## Run it

Requirements are a Rust toolchain and an installed `zsh`.

```sh
./scripts/start-keel.sh
```

The launcher starts stock Zsh with [`examples/isolated/.zshrc`](examples/isolated/.zshrc), which loads the small bridge in [`zsh/keel.zsh`](zsh/keel.zsh). Keel does not patch, rebuild, replace, or reinstall Zsh.

The editor supports normal typing, Unicode-safe cursor movement, word editing, kill/yank operations, history delegation, transient accepted and interrupted lines, a selectable completion overlay, path-aware matching, and a blinking cursor that repaints independently at 60 Hz. Enter returns the line to Zsh for execution, Escape returns it to stock ZLE, and Ctrl-C preserves the typed line as a transient prompt before starting a fresh Keel prompt.

## Architecture

The boundary stays small because stock Zsh does not expose a public API for replacing its whole ZLE input loop:

```text
stock Zsh ZLE
    │  one widget passes BUFFER, CURSOR, prompt, cwd
    ▼
Rust editor process
    │  input, editing, completion, scene, animation
    ▼
one ratatui Buffer and one diffed ANSI renderer
    │  prompt, command, overlay, and cursor are Elements
    ▼
terminal
```

The widget and editor exchange one text record over stdout, so there is no binary ABI and no private Zsh build. The renderer opens the terminal already owned by Zsh, saves the active cursor position as its origin, and writes only cell diffs; it never captures or re-renders shell output through a pass-through layer.

Completions use two sources. A Rust filesystem source responds immediately for paths and common path-taking commands, while an optional child Zsh provider runs the user's completion functions inside a short-lived PTY and sends candidates over fd 3. Rust merges, deduplicates, fuzzy-ranks, highlights, compacts, and paints every candidate, so shell compatibility remains available without moving the UI or hot path into shell code.

Every visible surface implements the same `Element` contract and paints into the same frame. Adding a future menu, preview, notification, or inline diagnostic therefore extends scene composition rather than introducing another renderer or coordinate system.

## Development

```sh
cargo fmt --all -- --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
python3 scripts/terminal-harness.py
for file in zsh/*.zsh; do zsh -n "$file"; done
bash -n scripts/*.sh
```

[`scripts/terminal-harness.py`](scripts/terminal-harness.py) allocates a real bidirectional pseudo-terminal and drives Zsh in both directions. It checks redraw origin, independent cursor repainting, path completion, transient lines, Escape behavior, command execution, and shell return.

The main code is split by ownership:

- [`src/editor.rs`](src/editor.rs) coordinates the loop; child modules own editing, navigation, completion updates, and view state.
- [`src/scene.rs`](src/scene.rs) defines the homogeneous scene contract and delegates to the canvas, elements, overlay, and transient layouts.
- [`src/completion.rs`](src/completion.rs) owns the Rust completion pipeline, path matching, and the optional Zsh provider boundary.
- [`src/render.rs`](src/render.rs) diffs ratatui cells and writes ANSI updates, with encoding kept beside it.
- [`src/input.rs`](src/input.rs) owns the direct-TTY input loop, raw mode, terminal size, and key decoding.
- [`zsh/keel.zsh`](zsh/keel.zsh) is the shell-side bridge, while [`zsh/keel-provider.zsh`](zsh/keel-provider.zsh) is the isolated completion adapter.

## License

Keel is licensed under MIT OR Apache-2.0.
