# Keel

Keel is a native frontend engine for `zsh`.

The current repository implements the foundational command-read loop:

- `zsh` remains the backend for parsing and executing accepted commands.
- `keel-shell` owns the active prompt/editor surface during command entry.
- the Rust runtime renders the prompt, buffer, transient submit state, and cursor
- the shell session loop hands accepted commands back to normal `zsh` execution

This is the core engine path for phases 1-3 groundwork. Native history search, completions, autosuggestions, and richer prompt providers are not the focus of the current implementation.

## Workspace

- `crates/keel-core`: shared config, timing, frame, input, and session types
- `crates/keel-editor`: editable command buffer and key handling
- `crates/keel-render`: frame construction and diff generation
- `crates/keel-runtime`: runtime state machine for prompt editing and submission
- `crates/keel-terminal`: terminal I/O, patch application, input parsing, and session loop
- `crates/keel-prompt`: prompt layout model
- `crates/keel-shell`: shell snapshot collection
- `crates/keel-zsh-bridge`: native zsh-facing ABI boundary
- `crates/keel-cli`: standalone frontend binary and future control-plane commands
- `crates/keel-history`: reserved crate for native history work
- `crates/keel-complete`: reserved crate for native completion work
- `crates/keel-devtools`: reserved crate for replay, inspection, and diagnostics
- `shell/zsh`: shell activation shim and session helpers
- `scripts`: build, launch, and smoke-test entrypoints
- `zsh-module`: native zsh module sources

## Current Behavior

- Keel renders the active prompt as `keel> ` by default.
- On submit, Keel renders a transient prompt of `> `, restores the terminal, and returns the accepted command to `zsh`.
- Command output runs outside the frontend renderer.
- The next Keel prompt resumes after command completion.
- `Ctrl-C` cancels the active Keel buffer instead of dropping into default prompt editing.

## Running

Build the module and frontend binary:

```bash
./scripts/build-zsh-module.sh
```

Start a Keel-owned shell session:

```bash
./scripts/start-keel-session.sh
```

That launcher now defaults to compatibility mode:

- it preserves your normal shell environment by sourcing `~/.zprofile` and `~/.zshrc`
- it still gives Keel ownership of the command-entry frontend

Start a guaranteed-clean frontend session instead:

```bash
KEEL_MINIMAL_SHELL=1 ./scripts/start-keel-session.sh
```

Load the shim manually inside an existing shell:

```bash
source ~/Projects/keel/shell/zsh/keel.zsh
```

Disable prompt auto-entry and keep only the explicit binding:

```bash
KEEL_AUTO_START=0 source ~/Projects/keel/shell/zsh/keel.zsh
```

## Validation

Run the module smoke test:

```bash
./scripts/test-zsh-prototype.sh
```

Run the Rust tests:

```bash
cargo test
```

## Notes

- Fullscreen terminal takeover now works for standard TUI flows like `less`.
- Keel is still a foundational engine build. Some app-specific TUI integrations may still need terminal-compatibility work.
