# Keel

Keel is a terminal augmentation prototype. Zsh remains responsible for the prompt lifecycle, line editing, keymaps, history, command acceptance, execution, and scrollback. Keel observes each completed ZLE redisplay, renders an optional ratatui surface offscreen, and writes one bounded ANSI transaction below the host cursor.

The MVP keeps the normal prompt visible and adds a Rust-rendered autocomplete box whenever the current line is non-empty. The completion data is deliberately small and deterministic so the host/render boundary can be tested before a real completion provider is added.

The native module requires the Keel-enabled Zsh built by `scripts/build-zsh.sh`; the system `/bin/zsh` is not modified. Start the isolated demo with:

```sh
./scripts/start-keel.sh
```

Run the complete Rust, native, and PTY test suite with:

```sh
./tests/run.sh
```

The implementation is intentionally free of a daemon, PTY wrapper, alternate screen, scrollback manager, async runtime, and shell-side editor state.
