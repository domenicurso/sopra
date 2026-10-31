# Rust prompt widgets

Put a self-contained Rust file in `~/.config/sopra/widgets/` (or
`$XDG_CONFIG_HOME/sopra/widgets/`). Its filename becomes its template token:
`uptime.rs` supplies `{uptime}` without a registration step. Names can contain
ASCII letters, numbers, and underscores; built-in widget names are reserved.
Only widgets referenced by a prompt token or conditional are loaded.

## A dynamic widget

Save this as `uptime.rs`:

```rust
struct Uptime(std::time::Instant);

impl sopra::Widget for Uptime {
    fn render(&mut self, context: &sopra::Context) -> String {
        let label = context.options.get("label").map_or("up", String::as_str);
        format!("{label} {}s", self.0.elapsed().as_secs())
    }
}

fn main() -> std::io::Result<()> {
    sopra::run(Uptime(std::time::Instant::now()))
}
```

Then use it in `config.toml`:

```toml
[prompt]
left = "{path}\n$ "
right = "[if uptime]{uptime}[/]"
transient = "$ "

[widgets.uptime]
label = "session"
format = "[dim]{text}[/]"
```

The widget supplies plain text. Apply styles around its token or with its
`format` option, which substitutes `{text}` and accepts the normal prompt tags.
Brackets in widget output are escaped rather than interpreted as styling.
An empty result stays empty, so `[if uptime]` can hide adjacent separators.

## Context and state

The embedded `sopra` module exposes:

- `Context.cwd: PathBuf` is the current shell directory.
- `Context.options: BTreeMap<String, String>` contains this widget’s TOML
  settings. String values are decoded; other values use TOML notation.
- `Context.environment: BTreeMap<String, String>` is the shell’s latest exported
  environment, including command context supplied by the bridge.
- `Widget::render(&mut self, &Context) -> String` supplies the displayed text.

The widget instance stays alive across command submissions, so struct fields
can hold timers, selection state, or cached results. Context refreshes when the
shell requests another edit. Use `context.cwd` and `context.environment` for
current state; the child process’s own environment is its startup snapshot.
The API currently uses UTF-8 text and the Rust standard library; separate Cargo
packages and external crate dependencies are not supported by this file format.

## Interaction

An interactive widget implements `interactive() -> bool`, `active() -> bool`,
and `key(&mut self, &str, &Context) -> sopra::Action`. All have inactive defaults.
Ctrl-G focuses the first available interactive widget; pressing it again cycles
to another one. With a single interactive widget, it cancels focus. Active
widgets receive keys before command editing.

Key names include `PromptWidgets` (activation), `Tab`, `BackTab`, `Enter`,
`Escape`, `Cancel`, `Left`, `Right`, `Up`, and `Down`. Printable keys use their
Rust debug spelling, such as `Character('a')`. A minimal parent-folder picker:

```rust
#[derive(Default)]
struct Parent { active: bool }

impl sopra::Widget for Parent {
    fn render(&mut self, context: &sopra::Context) -> String {
        context.cwd.parent().map_or(String::new(), |p| p.display().to_string())
    }
    fn interactive(&self) -> bool { true }
    fn active(&self) -> bool { self.active }
    fn key(&mut self, key: &str, context: &sopra::Context) -> sopra::Action {
        match key {
            "PromptWidgets" => self.active = true,
            "Escape" | "Cancel" => self.active = false,
            "Enter" => {
                self.active = false;
                if let Some(parent) = context.cwd.parent() {
                    return sopra::Action::ChangeDirectory(parent.to_owned());
                }
            }
            _ => {}
        }
        sopra::Action::None
    }
}

fn main() -> std::io::Result<()> { sopra::run(Parent::default()) }
```

`ChangeDirectory` asks the owning Zsh to change directory while retaining the
command buffer. Use `Action::None` for a key that only changes widget state.

## Compilation and failures

Sopra provides the SDK when compiling a file with `rustc --edition=2024 -O`.
A working Rust toolchain must be on `PATH`. Compiled binaries are cached by
source and SDK contents under `$XDG_CACHE_HOME/sopra/widgets/`, or
`~/.cache/sopra/widgets/`. Unchanged files reuse the cached binary.

Each loaded custom widget owns a background thread and a separate persistent
process. The thread checks source changes and refreshes output every 250 ms;
the terminal renders cached text without waiting for compilation. New files
are discovered at the next prompt. Source changes rebuild and restart that
widget, resetting its state while leaving the editor running.

Compilation has a 30-second deadline. Each render or key callback must respond
within 100 ms; perform slow work in your own background thread and return a
cached value. Invalid output, a timeout, or a crash stops the widget until its
source changes. The prompt shows `widget error`, and the cache’s `errors.log`
records the cause and the compiler log location. Terminal control characters
and responses over 64 KiB are rejected. These processes run with your user’s
permissions; this is failure isolation, not a sandbox.

## Built-in ownership

Every built-in lives in `src/prompt/widgets/builtins/<name>.rs` and owns its
rendering and interaction state. `build.rs` discovers those files and generates
the factory registry during compilation. A new built-in implements the internal
`Widget` trait and a `create(&WidgetContext) -> Box<dyn Widget>` factory in its own file.
The context contains the working directory and that widget’s own TOML options,
so adding an option does not require a config field or a Zsh mapping.
The template renderer owns conditional blocks, escaping, and surrounding
styles; the registry does not contain widget-specific branches.
