# Architecture

Keel augments an interactive Zsh session instead of replacing it. The stock shell owns the visible prompt and the editable line, while Keel owns only the optional surface it paints into the terminal. This boundary lets normal Zsh behavior continue to handle history, cursor movement, accept-line, command execution, output, and terminal scrollback.

## Ownership

`zsh/keel.zsh` loads the module and installs the two ZLE lifecycle hooks. It contains no editor state machine, redraw loop, process manager, or prompt projection.

The C shim is the only code that knows the Zsh ABI. It registers the module and line hooks, receives the patched ZLE redisplay callbacks, snapshots the host line and terminal geometry, and writes Rust's returned byte payload to `SHTTY`. It does not decide layout or compose UI.

`keel-core` normalizes host data and keeps the session model. Its buffer and cursor types are grapheme-aware, but the MVP observes the line Zsh edits rather than trying to replace ZLE's editing engine.

`keel-ui` composes a measured component tree from ratatui-compatible components. `keel-renderer` turns that tree into an offscreen ratatui buffer, compares it with the previous frame, and encodes a bounded region transaction. `keel-scheduler` defines invalidation and frame timing without introducing a background runtime.

## Redisplay lifecycle

The Keel-enabled Zsh calls the module before its normal redraw. Keel clears the previous surface using cursor save/restore and targeted row erases. Zsh then draws its ordinary prompt and editable line. The post-redraw callback receives the final visual cursor position, so Rust can anchor the next surface to the actual host cursor rather than guessing from prompt strings.

Rust mirrors that snapshot, builds the component tree, measures it against the terminal width and row budget, paints an offscreen buffer, diffs it, and returns one ANSI payload. The payload hides the cursor while it paints, clears rows that are no longer used, restores the host cursor, and never enters the alternate screen or clears the terminal.

On accept, Zsh's normal `accept-line` remains in charge. The line-finish hook removes Keel's surface before command output begins, so the command and its output use ordinary shell semantics. Cancel and empty input likewise follow native ZLE behavior; Keel only tears down its overlay and does not print a synthetic prompt.

## Build boundary

The patch in `patches/zsh-5.9-keel-redraw.patch` adds a small callback ABI to a private Zsh 5.9 build. `scripts/start-keel.sh` uses that build with an isolated `ZDOTDIR`, which makes the demo reproducible without changing the user's installed shell; its final `exec` means the launcher itself does not become a second shell process. `install.sh` delegates to `scripts/install-keel.sh`, which installs the same private Zsh, module, loader, and stable `bin/keel` wrapper under a user-owned prefix so a terminal profile can launch Keel as its root interactive process without changing `chsh`. The installed wrapper uses a dedicated startup directory and only imports `~/.zshrc` when `KEEL_SOURCE_USER_RC=1`, so shell startup bugs cannot silently prevent the module from loading.

The loader exposes `keel status`, `keel enable`, and `keel disable` as a shell-native command surface. The wrapper also accepts `keel status` outside an active session for installation diagnostics, but session mutation stays in the shell function because a child process cannot change its parent Zsh's module state.

The C shim and Rust module both require ABI version 1, and the patched Zsh exports the matching version symbol before the module can register widgets or redraw hooks. The integration suite loads the module in the patched Zsh and verifies that the stock `/bin/zsh` rejects it, so a mismatched shell cannot silently run against the wrong callback layout.
