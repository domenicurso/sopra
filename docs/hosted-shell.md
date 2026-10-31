# Hosted shells and scrollback

Run `sopra shell` to start Zsh with ordinary wheel scrollback and command-line
mouse selection. It inherits your working directory, loads your `.zshenv` and
`.zshrc`, then enables the Sopra bridge if necessary. `scripts/start.sh` uses
this mode with the repository's isolated development configuration.

Wheel scrolling and Page Up/Page Down move through the original terminal output.
The prompt scrolls with the output; there are no block headers, controls, or
pinned input. Typing returns to the live prompt and preserves your draft.
Command-line clicks and selection work at the live bottom as before.

```toml
[session]
scrollback_lines = 10000
```

This positive integer bounds retained lines for the hosted session. Output stays
in memory until the session closes; persisted Zsh command history is separate.
The viewport belongs to Sopra rather than the outer terminal, which allows plain
wheel scrolling while the editor receives mouse clicks without Shift overrides.
The bridge-only setup remains available, but cannot route wheel events into
native terminal scrollback while mouse reporting is active.

Foreground alternate-screen applications receive their requested mouse events
instead of scrolling shell output. Keyboard input, PTY resizing, and job signals
continue to reach the child. Sopra restores terminal modes on exit and hangs up
the shell and foreground jobs on shutdown.

The host renders core xterm text, including ANSI colors and attributes, and
forwards palette queries, titles, and clipboard writes. Graphics protocols and
terminal-specific extensions are not supported.
