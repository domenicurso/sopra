# Keel

Keel is a Rust augmentation layer for zsh. It adds a small, styled status UI
around the existing shell prompt while zsh remains responsible for the
terminal, ZLE editing, cursor movement, wrapping, resize, cancellation,
acceptance, history, and command execution.

The current prototype uses one long-lived Rust renderer per shell. Zsh sends a
compact snapshot over a private request/response protocol from
line-pre-redraw; Rust builds a generic component tree, paints it into an
offscreen ratatui Buffer, and returns a structured host plan containing a
zsh-safe RPROMPT fragment plus an optional cursor decoration. There are no
widget wrappers, redraw loops, alternate-screen sequences, or cursor-motion
sequences in prompt text, so the normal shell continues to own the interactive
line.

## Run The Prototype

```bash
./scripts/start-keel-augment.sh
```

The script builds the renderer and opens a clean interactive zsh. Type, move
the cursor, resize the terminal, press Ctrl-C, and run commands. The badge on
the right is recomputed from the live ZLE snapshot, while normal shell
behavior continues underneath it. The badge includes the active keymap,
grapheme count, cursor index, and the last command status.

The safe cursor-relative mode is opt-in:

```bash
KEEL_AUGMENT_CURSOR_MODE=highlight \
KEEL_AUGMENT_CURSOR_STYLE=bar \
./scripts/start-keel-augment.sh
```

`native` mode changes only the terminal cursor shape. `highlight` mode also
asks zsh to highlight the grapheme under the cursor through `region_highlight`;
zsh still calculates the line position and performs every redisplay. `off`
leaves the terminal cursor untouched. Keel never embeds cursor movement in
`RPROMPT`, because zsh would then lose track of the prompt width and could
overwrite the decoration on the next redisplay.

To load it into an existing interactive zsh:

```zsh
cargo build -p keel-augment
source /absolute/path/to/keel/zsh/keel-augment.zsh
```

Disable it with keel-augment-disable and inspect the boundary and renderer
cache with keel-augment-status. The optional KEEL_AUGMENT_THEME=mono or
KEEL_AUGMENT_THEME=amber environment setting selects a built-in theme.

## Architecture

```text
zsh .zshrc
    |
    v
keel-augment.zsh -> persistent keel-augment --server process
    |                         |
    | HostSnapshot             v
    +--------------------> keel-core state/types
                              |
                              v
                    keel-ui component tree
                              |
                              v
                    ratatui offscreen Buffer
                              |
                              v
                    keel-renderer ANSI/zsh fragment
                              |
                              v
                  zsh host decoration plan
                    /                \
               RPROMPT       cursor style/highlight
```

The shell adapter only captures state, applies the returned host plan, installs
lifecycle hooks, and restores the user's prompt on disable. Rust owns
composition, measurement, and the decision about which cursor decoration is
appropriate, but it deliberately does not own the terminal or the shell's
editing buffer. That ownership split is what lets Keel augment the prompt
without reproducing ZLE's wrapping, cursor, history, and command semantics.

This prototype intentionally does not include completions, AI, plugins,
mouse input, scrollback management, raw mode, alternate-screen ownership, or
a native zsh module. Those are follow-on capabilities that must preserve the
same host-ownership boundary.
