# Architecture

Keel augments an interactive Zsh session instead of replacing it. The stock shell owns the visible prompt and the editable line, while Keel owns only the optional surface it paints into the terminal. This boundary lets normal Zsh behavior continue to handle history, cursor movement, accept-line, command execution, output, and terminal scrollback.

## Ownership

`zsh/keel.zsh` loads the module, installs the ZLE lifecycle hooks, binds the small set of Keel widgets, and brokers completion capture. It contains no renderer, prompt projection, or independent editor; one completion context is captured in a short-lived `zpty`, cached, and filtered locally so the user's Zsh process remains the host of the live line.

The C shim is the only code that knows the Zsh ABI. It registers the module and native widgets, receives the patched ZLE redisplay callbacks, snapshots the host line and terminal geometry, and writes Rust's returned byte payload to `SHTTY`. It does not decide layout or compose the UI. The shell side calls the normal completion widget in a forked `zpty`, intercepts `compadd` only in that child, and sends bounded label, description, and candidate records back to the native module.

`keel-core` normalizes host data and keeps the session model. Its buffer and cursor types are grapheme-aware, and its native fuzzy matcher re-ranks cached provider records without rerunning Zsh, but the MVP observes the line Zsh edits rather than trying to replace ZLE's editing engine.

`keel-ui` composes a measured component tree from ratatui-compatible components. `keel-renderer` turns that tree into an offscreen ratatui buffer, compares it with the previous frame, and encodes a bounded region transaction. `keel-core` applies `neo_frizbee` ranking to the real Zsh completion records before they reach the component tree. `keel-scheduler` defines invalidation and frame timing without introducing a background runtime.

## Redisplay lifecycle

The Keel-enabled Zsh calls the module before its normal redraw. Keel clears the previous surface using cursor save/restore and targeted row erases. Zsh then draws its ordinary prompt and editable line. The post-redraw callback receives the final visual cursor position, so Rust can anchor the next surface to the actual host cursor rather than guessing from prompt strings. A `line-pre-redraw` hook starts or reuses one completion request for the current broad context; later edits hit the shell cache and native fuzzy filter, while the file-descriptor callback applies an uncached response through a ZLE widget only if the line, cursor, and working directory still match the request snapshot.

Rust mirrors that snapshot, builds the component tree, measures it against the terminal width and available rows above or below the host cursor, paints an offscreen buffer, diffs it, and returns one ANSI payload. The payload hides the cursor while it paints, clears rows that are no longer used, restores the host cursor, and never enters the alternate screen or clears the terminal. The popup has at most twelve items and keeps one visible row in the direction of selection travel. Its scrollbar overwrites the right border, while the count and completion time occupy the bottom border. The C shim independently selects a block cursor while Keel is active, so cursor shape is stable even when the popup is absent.

On accept, Zsh's normal `accept-line` remains in charge. Tab asks the native widget for the selected replacement and writes that complete line into ZLE; Enter then accepts the line normally. The line-finish hook removes Keel's surface before command output begins, so the command and its output use ordinary shell semantics. Escape dismisses the current popup, Ctrl-C clears the current ZLE line without accepting it, and empty Enter only requests a redisplay, so no path prints a synthetic prompt.

## Build boundary

The patch in `patches/zsh-5.9-keel-redraw.patch` adds a small callback ABI to a private Zsh 5.9 build. `scripts/start-keel.sh` uses that build with an isolated `ZDOTDIR`, which makes the demo reproducible without changing the user's installed shell; its final `exec` means the launcher itself does not become a second shell process. `install.sh` delegates to `scripts/install-keel.sh`, which installs the same private Zsh, module, loader, and stable `bin/keel` wrapper under a user-owned prefix so a terminal profile can launch Keel as its root interactive process without changing `chsh`. The installed wrapper uses a dedicated startup directory and only imports `~/.zshrc` when `KEEL_SOURCE_USER_RC=1`, so shell startup bugs cannot silently prevent the module from loading.

The loader exposes `keel status`, `keel enable`, and `keel disable` as a shell-native command surface. The wrapper also accepts `keel status` outside an active session for installation diagnostics, but session mutation stays in the shell function because a child process cannot change its parent Zsh's module state.

The C shim and Rust module both require ABI version 1, and the patched Zsh exports the matching version symbol before the module can register widgets or redraw hooks. The integration suite loads the module in the patched Zsh and verifies that the stock `/bin/zsh` rejects it, so a mismatched shell cannot silently run against the wrong callback layout.
