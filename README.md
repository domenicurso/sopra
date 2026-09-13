# Keel

<div align="center">

**A native UI layer for interactive Zsh.**

Keel adds Rust-rendered terminal surfaces to a normal Zsh session while keeping Zsh in charge of the prompt, line editing, history, command execution, and scrollback.

</div>

Keel is a terminal augmentation prototype. The current MVP renders a small autocomplete surface below the active line by asking Zsh's own completion system for matches in one short-lived `zpty` per completion context, then paints those matches through the native renderer without asking ZLE to draw a second prompt. The captured set is cached and fuzzy-filtered locally while the line changes, so warm edits do not start another Zsh provider or block on a new process.

- [Try the isolated demo](#try-the-isolated-demo)
- [Install a user-owned shell](#install-a-user-owned-shell)
- [Understand the architecture](#how-it-works)
- [Run the test suite](#development)

## What it does

- Renders a Rust and [ratatui](https://ratatui.rs/) suggestion popup below the host cursor without replacing Zsh's editor.
- Keeps normal Zsh behavior intact, including prompt rendering, cursor movement, command acceptance, execution, history, and terminal scrollback.
- Measures the surface against the terminal geometry and writes only a bounded ANSI transaction, so overlays can be cleared without an alternate screen or a scrollback manager.
- Tracks Unicode input by grapheme and display width, so the host line can contain multi-codepoint characters without making the Rust model lose its place.
- Captures each broad completion context once, then performs cached fuzzy filtering and ranking in native code so the common edit path is sub-millisecond.
- Exposes a `keel` command with `status`, `enable`, `disable`, and `help` subcommands inside an active session.

### Who is it for?

Keel is for people who want to experiment with rich shell interfaces while preserving the behavior users already rely on, and for developers who want a small Rust-native foundation for terminal surfaces rather than another shell-side editor implementation.

The current demo is deliberately modest: any `compdef` available after `compinit` can provide the popup entries, including descriptions from `compadd -d`. Keel displays those entries below the line, keeps selection in Rust, and writes a selected replacement back into ZLE only when Tab is pressed.

## Installation

> [!IMPORTANT]
> Keel builds a private, patched Zsh 5.9 because the native module depends on a small redraw callback ABI. The system Zsh is not modified, and the build scripts currently support macOS and Linux.

### Requirements

You need a Rust toolchain, a C compiler, `make`, `curl`, `tar`, and `patch`. The full integration suite also uses [Expect](https://core.tcl-lang.org/expect/index), because it drives a real interactive Zsh session through a pseudo-terminal.

### Try the isolated demo

From the repository root, run:

```sh
./scripts/start-keel.sh
```

The first run downloads and builds the private Zsh, compiles the Rust module, initializes Zsh's standard completion definitions, creates a temporary startup directory under `target/`, and launches the demo shell. It does not edit your dotfiles or change your default shell.

The launcher replaces itself with the Keel-enabled Zsh, but it cannot replace the shell that invoked it. Use `exec` when the current terminal shell should be replaced too:

```sh
exec ./scripts/start-keel.sh
```

### Install a user-owned shell

To keep a stable Keel installation outside the build tree, run the one-command installer from the repository root:

```sh
./install.sh
```

The installer puts the patched Zsh, native module, loader, and isolated startup files under `$HOME/.local/keel`. It does not edit `.zshrc`, `.zprofile`, or `/etc/shells`, and it does not call `chsh`. Set `KEEL_INSTALL_PREFIX` when you want a different user-owned location:

```sh
KEEL_INSTALL_PREFIX="$HOME/.local/keel-dev" ./install.sh
```

Launch the installed shell directly while testing it:

```sh
"$HOME/.local/keel/bin/keel" -il
```

For a terminal profile, use `$HOME/.local/keel/bin/keel -il` as the profile command. That makes the private patched Zsh the root interactive process for the terminal while preserving ordinary Zsh execution, history, and scrollback.

The installed wrapper reuses `~/.zshenv`, `~/.zprofile`, and `~/.zlogin`, but leaves `~/.zshrc` opt-in because interactive rc files commonly initialize plugins or make network calls. Enable it only after checking that it is safe for the private shell:

```sh
KEEL_SOURCE_USER_RC=1 "$HOME/.local/keel/bin/keel" -il
```

Remove the user-owned installation without touching any shell dotfiles:

```sh
./uninstall.sh
```

Use the same `KEEL_INSTALL_PREFIX` override when removing an installation stored somewhere else. The uninstaller removes a prefix only when it contains Keel's install marker, so it refuses to delete an unrelated directory.

### Build from source

The normal source build is handled by the demo and install scripts. To build the patched shell and native module without launching a session, run:

```sh
./scripts/build-keel.sh
```

The resulting module is written to `target/debug/keel.so`, and the patched Zsh is installed under `target/keel-zsh` unless `KEEL_ZSH_PREFIX` overrides that location.

## Configuration

Keel does not have a larger user-facing configuration system yet. Once `zsh/keel.zsh` has loaded the module, the shell-native `keel` command is available:

```zsh
keel status    # report whether this interactive shell has Keel hooks
keel enable    # load the module and install the hooks
keel disable   # remove the hooks and unload Keel for this session
keel help      # show the command syntax
```

The installed `bin/keel` wrapper also accepts `keel status` outside an active session and reports where Keel is installed. `enable` and `disable` must run inside the managed shell because an executable cannot change the module state of the shell that launched it.

The integration only activates in an interactive shell with `KEEL_MODULE_PATH` set. Sourcing the loader from an ordinary stock Zsh is safe when that variable is absent; the loader returns without installing hooks. A normal existing Zsh setup can keep control of `compinit`; the isolated demo and installed shell initialize it explicitly so the autocomplete MVP works without extra configuration.

## How it works

Keel augments an interactive Zsh session instead of taking ownership of it. The patched shell remains the source of truth for the visible prompt and editable line, while Keel owns only the optional surface it paints into the terminal.

```text
Zsh ZLE redisplay
        |
        v
patched redraw hooks -> C ABI snapshot -> Rust app state
                                              |
                                              v
                                   ratatui component tree
                                              |
                                              v
                                  offscreen buffer and diff
                                              |
                                              v
                                      bounded ANSI patch
```

### Ownership boundary

`zsh/keel.zsh` loads the native module and installs the `line-init` and `line-finish` hooks. The C shim is the only layer that knows the Zsh ABI: it receives the patched redisplay callbacks, snapshots the current line and terminal geometry, registers the small native widgets, and writes Rust's returned bytes to the terminal. It does not decide layout or compose the UI. The shell loader runs the real completion widget once in a bounded `zpty` for a broad context, caches the `compadd` records, rejects stale responses against the line snapshot, and hands one protocol payload to the native module; Rust then filters later edits locally.

The Rust crates keep the rest of the work separated. `keel-core` normalizes host data and maintains the session model, `keel-ui` measures and paints components, `keel-renderer` turns those components into a diffed ANSI transaction, and `keel-scheduler` defines invalidation and frame timing without starting a background runtime.

### Terminal behavior

Before Zsh redraws, Keel clears the previous surface. After Zsh has drawn its ordinary prompt and line, the post-redraw hook receives the actual visual cursor position, so the next surface can anchor to the host cursor rather than guessing from prompt strings.

On accept, Zsh's normal `accept-line` remains in charge. The line-finish hook removes Keel's surface before command output begins, so the command and its output use ordinary shell semantics. Tab accepts a selected completion by updating the ZLE buffer, while Enter executes the resulting line. Escape dismisses the popup, Ctrl-C clears the current line in place, and empty Enter only redisplays it; none of these paths prints a synthetic prompt. Keel never enters the alternate screen, clears the terminal, rewrites scrollback, wraps the shell in another PTY, or starts a daemon.

For the lower-level rendering contract, see [the architecture notes](docs/architecture.md) and [the renderer notes](docs/renderer.md).

## Development

Run the complete validation suite with:

```sh
./tests/run.sh
```

The suite checks shell-script syntax, formatting, locked workspace tests, Clippy warnings, native module loading, rejection by stock Zsh, installation behavior, and the interactive PTY flow for accept, cancel, resize, and unload.

For focused Rust checks, use:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

### Repository layout

- `crates/keel-core` contains the host snapshot, grapheme-aware line model, application state, and completion selection state.
- `crates/keel-ui` contains the measured component API, terminal-palette style tokens, and suggestion popup.
- `crates/keel-renderer` contains the offscreen ratatui buffer, frame diff, and bounded ANSI writer.
- `crates/keel-scheduler` contains invalidation and frame-clock behavior for the host integration.
- `native` contains the C loadable-module shim and the checked Zsh ABI boundary.
- `patches` contains the Zsh 5.9 redraw-hook patch used by the private shell build.
- `zsh` contains the shell-side loader and lifecycle commands.
- `install.sh` and `uninstall.sh` manage the user-owned prefix; `scripts` contains their build, removal, and demo implementation scripts.
- `tests` contains Rust-adjacent integration checks and the Expect-driven native session test.

## Current limitations

- Completion capture follows Zsh's installed completion functions, is bounded to 512 records, and uses [`neo_frizbee`](https://docs.rs/neo_frizbee/latest/neo_frizbee/), the matching engine used by [`fff`](https://github.com/dmtrKovalenko/fff), to rank the captured labels while preserving `compadd` descriptions. Cache hits report `0ms` because only native filtering runs.
- The sub-millisecond target applies to cached filtering and rendering. A cache miss executes arbitrary Zsh completion code in an isolated `zpty`, so its first-result time remains provider-dependent and cannot be guaranteed below 1ms for every completion function.
- Zsh still owns keyboard input and the editable buffer; Keel adds navigation and Tab insertion widgets while leaving command parsing, history, and execution in ZLE.
- Running Keel requires the patched Zsh 5.9 build, which is why the project builds and ships its own private shell instead of loading into `/bin/zsh`.
- The native build scripts currently implement Darwin and Linux link steps; other host operating systems are rejected explicitly.

## License

Keel is licensed under [MIT](https://opensource.org/license/mit/) OR [Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0).
