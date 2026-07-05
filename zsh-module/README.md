# Native zsh Boundary

Keel is a frontend for zsh, not a replacement for zsh. The native module is therefore the primary integration point: zsh owns shell semantics and execution, while Keel owns the command-line surface during command read.

## Current shape

- `crates/keel-zsh-bridge` provides the Rust-owned C ABI.
- `zsh-module/keel_module.c` binds that ABI to zsh's module lifecycle and ZLE widgets.
- `shell/zsh/keel.zsh` activates the module and installs the default bindings.

The module currently provides:

- module initialization and shutdown
- editor activation and deactivation
- interactive command-read entry with prompt, buffer, and cursor inputs
- accepted-command return back to zsh
- widget-backed interception for `accept-line`
- a manual edit widget that does not auto-execute
- bridge-owned error reporting and string deallocation

## Build

```bash
./scripts/build-zsh-module.sh
```

That script prepares the required generated zsh headers from the configured zsh source tree, builds the Rust bridge, and links `target/zsh/keel.so`.

## Activate

```bash
source ~/Projects/keel/shell/zsh/keel.zsh
```

Default behavior:

- `Enter` routes the current line through Keel and then returns the accepted command to normal zsh execution.
- `Ctrl-X Ctrl-K` opens Keel without immediately executing the line.
- `Ctrl-C` cancels back to zsh and preserves the original buffer.
