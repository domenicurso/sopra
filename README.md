# Keel

Keel is a Rust augmentation layer for zsh. It adds UI around the existing
shell prompt while zsh remains responsible for the terminal, ZLE editing,
cursor movement, wrapping, resize, cancellation, acceptance, and command
execution.

The first prototype is intentionally narrow. Rust receives a snapshot of the
current ZLE line, composes a small ratatui widget into an offscreen buffer,
and returns a zsh-safe `RPROMPT` fragment. The shell calls that renderer from
the redraw hook and from tiny delegates around common built-in edit widgets;
each delegate invokes the original ZLE widget first and only requests a
normal prompt refresh afterward. The adapter never implements editing and
never writes directly to the tty.

## Run The Prototype

```bash
./scripts/start-keel-augment.sh
```

The script builds the renderer and opens a clean interactive zsh. Type, move
the cursor, resize the terminal, press Ctrl-C, and run commands. The badge on
the right updates as ZLE redraws the line, while the normal shell behavior
continues underneath it.

To load it into an existing interactive zsh:

```zsh
cargo build -p keel-augment
source /absolute/path/to/keel/zsh/keel-augment.zsh
```

Disable it with `keel-augment-disable` and inspect the boundary with
`keel-augment-status`.

## Architecture

```text
zsh/ZLE snapshot
       |
       v
keel-core host types
       |
       v
keel-ui component tree -> ratatui Buffer
       |
       v
keel-renderer -> zsh prompt fragment
       |
       v
zsh RPROMPT + native ZLE redisplay
```

This prototype intentionally does not include a daemon, alternate screen,
raw mode, direct cursor positioning, scrollback management, or a native zsh
module. Those are separate decisions that should only be made after the
augmentation boundary proves stable.
