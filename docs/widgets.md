# Rust prompt widgets

Built-in and user widgets implement the same `sopra::Widget` trait from
[`sopra-widget-sdk`](../crates/widget-sdk/src/lib.rs). Git, Git status, clock,
and path are real implementations of that public API, rather than special
cases in the renderer. Their source lives in
[`src/prompt/widgets/builtins/`](../src/prompt/widgets/builtins/).

Put a self-contained Rust file in `~/.config/sopra/widgets/` (or
`$XDG_CONFIG_HOME/sopra/widgets/`). Its filename becomes its widget name:
`uptime.rs` supplies `uptime` without registration. Names can contain ASCII
letters, numbers, and underscores; built-in names are reserved. Only widgets
referenced by a layout are loaded.

## Dynamic output

Save this as `uptime.rs`:

```rust
use sopra::{Context, Emphasis, Refresh, Text, Widget};
use std::time::{Duration, Instant};

struct Uptime(Instant);
impl Widget for Uptime {
    fn render(&mut self, context: &Context) -> Text {
        let label = context.option("label", "up");
        Text::new()
            .part(Emphasis::Dim, format!("{label} "))
            .push(format!("{}s", self.0.elapsed().as_secs()))
    }
    fn refresh(&self) -> Refresh {
        Refresh::Every(Duration::from_secs(1))
    }
}

fn main() -> std::io::Result<()> {
    sopra::run(Uptime(Instant::now()))
}
```

Use it in `config.toml`:

```toml
[prompt]
left = "[blue]$path[/blue]\n$ "
right = "[cyan]$uptime[/cyan]"
transient = "[dim]$[/dim] "

[widgets.uptime]
label = "session"
```

Widgets return `Text`: plain text parts with optional colorless `Emphasis`.
`.push(text)` appends ordinary text; `.part(Emphasis::Dim, text)` dims a part.
`Emphasis::Bold` emphasizes text and `Emphasis::Selected` reverses foreground and
background for selection. Every part inherits the template's color. Output is
never interpreted as template markup, and an empty result disappears along with
its connecting separators. See [configuration](../README.md#configuration).

## Context, state, and refresh

- `Context.cwd: PathBuf` is the current shell directory.
- `Context.options: BTreeMap<String, String>` contains this widget's TOML
  options. String values are decoded; other values use TOML notation. The
  host passes these through without interpreting widget-specific keys.
- `Context.environment: BTreeMap<String, String>` is the shell's latest exported
  environment, including command context supplied by the bridge.
- `Context::option(name, default)` reads an option with a default.
- `Widget::render(&mut self, &Context) -> Text` supplies the display.
- `Widget::refresh() -> Refresh` defaults to `OnContext`: render after context
  changes or an interaction. `Every(Duration)` also schedules periodic renders.

A custom widget instance stays alive across command submissions, so its fields
can hold timers or selection state. Built-in instances also stay alive across edits.
Use the supplied context for fresh shell state; a custom process's own environment is its startup snapshot.
The SDK uses UTF-8 text and the Rust standard library. Separate Cargo packages
and external crate dependencies are not supported by the single-file loader.

## Interaction

Interactive widgets implement `interactive() -> bool`, `active() -> bool`,
and `handle(&mut self, Event, &Context) -> Action`. Defaults are inactive and
return `Action::Ignored`. Ctrl-G activates the first interactive widget and
cycles through others; with one widget, pressing it again cancels focus.
Active widgets receive typed `Event::Key(Key)` events before command editing.

```rust
use sopra::{Action, Context, Event, Key, Text, Widget};

#[derive(Default)]
struct Parent { active: bool }
impl Widget for Parent {
    fn render(&mut self, context: &Context) -> Text {
        context.cwd.parent()
            .map_or_else(String::new, |p| p.display().to_string()).into()
    }
    fn interactive(&self) -> bool { true }
    fn active(&self) -> bool { self.active }
    fn handle(&mut self, event: Event, context: &Context) -> Action {
        match event {
            Event::Activate => self.active = true,
            Event::Key(Key::Escape | Key::Cancel) => self.active = false,
            Event::Key(Key::Enter) => {
                self.active = false;
                if let Some(parent) = context.cwd.parent() {
                    return Action::ChangeDirectory(parent.to_owned());
                }
            }
            _ => return Action::Ignored,
        }
        Action::Consumed
    }
}
fn main() -> std::io::Result<()> { sopra::run(Parent::default()) }
```

`Action::Consumed` reports an event handled by the widget.
`Action::ChangeDirectory` asks Zsh to change directory while retaining the
command buffer. `Key` includes Tab, BackTab, Enter, Escape, navigation keys,
and `Character(char)`; no string parsing or Rust debug spellings are involved.

## Compilation and failures

The file compiler embeds the canonical SDK sources into its build directory;
built-ins link the same SDK as a workspace crate. There is one trait, context,
text model, event model, and action model. Built-in factories are discovered
by `build.rs`; the registry owns no widget-specific options or styling.

Custom files compile with `rustc --edition=2024 -O`, so a working Rust toolchain
must be on `PATH`. Binaries are cached by source and SDK contents under
`$XDG_CACHE_HOME/sopra/widgets/`, or `~/.cache/sopra/widgets/`.

Each loaded custom widget owns a background thread and a persistent process.
The thread checks source changes every 250 ms and schedules callbacks according
to `Refresh`. The terminal reads cached text without waiting for compilation
or callbacks. Use `sopra widgets list` to inspect discovery and runtime errors. After adding
references to new files in your config, run `sopra refresh`; existing instances
retain their state, and widgets removed from the layout stop. Changed files
rebuild and restart that widget, resetting its state while leaving the editor running.

Compilation has a 30-second deadline. Each render or event callback must respond
within 100 ms; slow work belongs in a widget-owned background thread that updates
a cached value. Invalid output, timeout, or crash stops the widget until its
source changes. The prompt shows `widget error`; the cache's `errors.log`
records the cause and compiler log location. Terminal control characters and
responses over 64 KiB are rejected. Processes run with your user's permissions;
this boundary isolates failures and does not sandbox code.
