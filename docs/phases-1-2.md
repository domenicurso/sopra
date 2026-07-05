# Phase 1 and 2 Status

This repository now satisfies the Phase 1 and Phase 2 goals from the architecture specification.

## Phase 1: Native Module Skeleton

Implemented:

- a compiled native zsh module at `target/zsh/keel.so`
- a dedicated `keel-zsh-bridge` crate with a stable C ABI surface
- explicit module initialization and shutdown boundaries
- explicit editor activation and deactivation boundaries
- a command-read ABI that accepts prompt, buffer, and cursor inputs
- accepted-command return back to zsh for normal execution
- panic containment at the Rust-to-C boundary
- bridge-owned error reporting and string lifecycle management
- ZLE widget registration for `keel-accept-line` and `keel-edit-line`
- module-backed shell activation through `shell/zsh/keel.zsh`

## Phase 2: Minimal Custom Editor

Implemented:

- a dedicated editor buffer crate with cursor-aware insertion and deletion
- cursor movement with home and end behavior
- command acceptance and cancellation
- a deterministic frontend runtime state machine with explicit handoff modes
- frame rendering with a custom visual cursor
- terminal raw-mode entry and restoration
- resize event handling
- a terminal session driver that returns control to zsh after acceptance or cancellation
- a same-process command-read flow from ZLE into the Rust frontend and back to zsh

## Validation

- `cargo test`
- `./scripts/build-zsh-module.sh`
- `./scripts/test-zsh-prototype.sh`

Interactive activation:

```bash
source ~/Projects/keel/shell/zsh/keel.zsh
```

Fresh shell entrypoint:

```bash
./scripts/start-keel-session.sh
```

Recommended `~/.zshrc` gating for Keel sessions:

```zsh
if [[ -z "${KEEL_MINIMAL_SHELL:-}" ]]; then
  source $(brew --prefix)/share/zsh-autosuggestions/zsh-autosuggestions.zsh
  source ~/headline.zsh-theme
fi
```

That guard should wrap frontend-only shell customizations. Keep backend shell behavior outside it.

With the default settings, each new prompt auto-enters Keel, so command editing happens in the Keel frontend instead of default ZLE. `Enter` in Keel accepts the line and hands it back to normal zsh execution. `Ctrl-C` clears the Keel buffer and stays inside Keel. `Ctrl-X Ctrl-K` still opens Keel manually when auto-start is disabled.

Known limitation: because phase 1 and 2 still rely on a ZLE takeover inside an already customized shell, full frontend replacement is not guaranteed in the presence of third-party prompt themes, autosuggestion layers, syntax-highlighting widgets, or command wrappers. Users may still observe a blank prompt line, an echoed accepted command, or other shell-specific residue. Keel now emits a generic warning during activation when it sees existing hook/widget-based frontend integrations instead of trying to special-case individual shell customizations.

Disable auto-start and keep only the explicit widget bindings:

```bash
KEEL_AUTO_START=0 source ~/Projects/keel/shell/zsh/keel.zsh
```

The source tree now follows the repository-organization section more closely:

- `crates/` contains focused Rust workspace crates with domain-split source modules
- `zsh-module/` contains the native zsh adapter and C header boundary
- `shell/` contains activation and shell-facing glue
- `docs/`, `tests/`, `examples/`, `assets/`, and `xtask/` are dedicated top-level areas
