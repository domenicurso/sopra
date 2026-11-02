<div align="center">
  <h1>Sopra</h1>
  <p><code>/ˈsoʊ.prə/</code></p>
  <p>An experimental line editor for interactive Zsh</p>
  <img width="700" alt="Sopra showing syntax highlighting and contextual completions" src="./assets/sopra.png">
</div>

**Sopra** provides interactive line editing, syntax highlighting, and contextual completions inside Zsh. Zsh keeps ownership of command execution, shell state, aliases, functions, and environment variables.

## Status
Sopra is early-stage software at version `0.1.0`. It currently targets interactive Zsh on Unix-like systems; continuous integration covers macOS and Linux. Editing behavior and configuration may change while the integration settles.

## Install

### Requirements

The installer needs Zsh, Bash, Rust and Cargo, `curl`, `tar`, and the `install` command. It builds Sopra locally, so the machine needs a working Rust toolchain and network access to download the source archive.

### Quick install

```sh
curl -fsSL https://raw.githubusercontent.com/domenicurso/sopra/main/install.sh | bash
```

The installer builds the ref named by `SOPRA_REF` (which defaults to `main`) and installs the private worker and Zsh bridge under `~/.local`. It does not edit shell startup files or add a public executable to `PATH`.

From an interactive Zsh, enable Sopra in the current shell with:

```sh
source "$HOME/.local/share/sopra/editor.zsh"
```

Add the `source` line to `~/.zshrc` if Sopra should load in future interactive shells. If you install with a custom `SOPRA_PREFIX`, source `share/sopra/editor.zsh` from that prefix instead. The one-line installer is a convenience path; use the source-build instructions below when you need to inspect or pin the checkout first.

### Install from source

```sh
git clone https://github.com/domenicurso/sopra.git
cd sopra
cargo build --release --locked --bin sopra-worker
export SOPRA_BIN="$PWD/target/release/sopra-worker"
source "$PWD/zsh/editor.zsh"
```

Keep `SOPRA_BIN` and the `source` command in your Zsh configuration if you want to run this checkout directly.

## Features
- Unicode-safe editing with history, word movement, kill/yank operations, and transient command lines.
- Press Ctrl+G to toggle command-line mouse editing, then drag to select, double-click a word, or triple-click the whole line. Esc returns to native terminal selection and scrolling. Ctrl-C copies selected text when the terminal supports clipboard writes; without a selection it interrupts the command.
- Live highlighting for commands, strings, variables, numbers, delimiters, and incomplete expressions.
- Contextual completion for shell commands, aliases, functions, variables, arrays, paths, options, values, and recursive CLI subcommands.

Sopra uses the terminal’s primary screen, so Ghostty retains native scrollback and search. Mouse reporting is enabled only during command-line mouse editing; leave that mode to scroll normally.

## Commands

| Command | Behavior |
| --- | --- |
| `sopra refresh` | Validate and reload config, retaining existing widget instances and history. |
| `sopra restart` | Replace the worker process and reset widget state. Shell history stays intact. |
| `sopra config edit` | Open config in `$VISUAL`, then `$EDITOR`, or `vi`; validate and refresh after the editor exits successfully. |
| `sopra config check` | Check TOML, templates, and widget references without changing the session or running widgets. |
| `sopra config path` | Print the canonical config path, respecting XDG settings. |
| `sopra config show` | Print the running session's effective config, including defaults; outside a session, read it from disk. |
| `sopra widgets list` | List built-in and custom widget sources, whether they are loaded, and runtime errors. |
| `sopra doctor` | Check worker connectivity, Zsh integration, config, and the Rust compiler. |

Refresh validates before changing the session. Invalid config leaves the working
settings in place; custom widgets compile asynchronously and report their own
errors. Restart also validates first, then replaces the process using `exec`, so
the shell-tracked worker PID stays the same while widget instances reset.

Commands target the current shell through its private exported socket address.
Run session-changing commands after submitting the command line: an actively
editing worker rejects them without disturbing the buffer. `doctor` can inspect
an editing worker. Config checks, file editing, and widget discovery also work
without an active session. The installer provides `sopra` in the installation
prefix's `bin` directory, and the Zsh integration exposes the same CLI through a
shell function.

Help and widget listings adapt to the terminal width, and `config show` highlights
TOML. Redirected output stays plain; `NO_COLOR` also disables terminal styling.

## Configuration

Configure Sopra in `~/.config/sopra/config.toml` (or `$XDG_CONFIG_HOME/sopra/config.toml`). Edit this file to change the prompt and editor options.

```toml
[prompt]
left = "[red]$user[/red] [dim]in[/dim] [bold blue]$path[/bold blue]\n[dim]$[/dim] "
right = "[red]$status[/red] · [dim]$duration[/dim] · [cyan]$git[/cyan]"
transient = "[dim]$[/dim] "
secondary = "[dim]>[/dim] "

[widgets.clock]
format = "%H:%M:%S"

[editor]
cursor_animation = true
syntax_highlighting = true
autosuggestions = true

[completion]
enabled = true
```

`config.toml` is the only configuration file. Run `sopra refresh` after editing it,
or use `sopra config edit` to open, validate, and refresh it in one step.
Templates use `$widget` references and matching style tags, such as
`[bold blue]$path[/bold blue]`. Tags can nest. Styles include colors, `bold`,
`dim`, `italic`, `underline`, and `reverse`. Use `$$` for a literal dollar sign
before a name, and `[[` for a literal opening bracket. A standalone `$` is literal.

Text between widget references is a connector: it appears only between nonempty
values. In the right prompt above, a successful status disappears without leaving
an initial ` · `; if only Git is available, there are no separators. Missing middle
values leave one connector, chosen from the text immediately after the preceding
visible value. Text before the first reference and after the last reference stays
literal, so prompt characters and final input spacing are preserved.

| Widget | Display |
| --- | --- |
| `path` | Current directory, abbreviated relative to your home directory. |
| `git` | Git branch and counts of changed/untracked files and commits ahead/behind; empty outside a repository. |
| `git_status` | The Git counts without the branch. |
| `clock` | Local time, refreshed while editing. |
| `status` | Exit status of the last command when it failed. |
| `duration` | Duration of the last command. |
| `user`, `host`, `shell` | Current user, host, and shell name. |

Use `\n` to start another prompt row. The right prompt stays on the first row,
and the command starts after the last row. Incomplete input stays in the editor,
with `secondary` on each continuation line; the primary prompt becomes transient
only when the entire command is submitted. An execution-disabled Zsh parser checks
continuations in a background worker; aliases that introduce unfinished syntax
still use the live shell's continuation handling.

Widgets inherit the color surrounding their reference. They may dim punctuation,
emphasize text, or reverse a selection, but they don't select their own colors.
`widgets.*` controls content and behavior: the clock's `format` uses `strftime`,
and Git accepts `label`, `changed`, `untracked`, `ahead`, `behind`, `open`, `close`,
and `separator`. Put Rust widgets in `~/.config/sopra/widgets/`; their filenames
become references automatically. See [the widget SDK](docs/widgets.md).

History autosuggestions show the rest of your most recent matching command as dim text. Up and Down cycle the matches while the completion menu is closed; when it is open, they move through its contextual completions. Full commands from history never appear in that menu; history only breaks ties between equally strong valid completions. Alt+Up and Alt+Down cycle history matches in either view. Right or End accepts the active history suggestion; Alt+Right accepts the next word. Escape dismisses the hint until you edit the command. Hints are clipped to the current screen row and never enter the command buffer until accepted.

Ctrl+R opens a separate history search, seeded with the current command. Type to filter by substring, use Up/Down or Ctrl+R to cycle matches, and Enter or Tab to copy a result into the command line without executing it. Escape or Ctrl+C cancels and restores the draft and cursor. Search works independently of completion and autosuggestion settings.

History search rows show their age beside the command when Zsh has a timestamp, or “History” when the time is unknown. Ages under a minute show seconds, such as `12s ago`. The inline suggestion shows the same metadata in dim italics after its text when there is room; metadata is never inserted into the command. Without a matching inline suggestion or focused completion, Up and Down recall command history in place; Down restores your draft after returning past the newest entry. Multiline cursor movement takes priority within an existing command.

Sopra reads the active Zsh history, including commands loaded from your history file and commands run in this session. It does not maintain a separate history file. If `HISTFILE` is unset, the bridge uses `${ZDOTDIR:-$HOME}/.zsh_history` and defaults to saving 10,000 entries. Existing history paths and limits, including an explicit `SAVEHIST=0`, are preserved. Commands are appended during the session, with timestamps; an existing `SHARE_HISTORY` setup keeps its live sharing behavior.

Set `editor.autosuggestions = false` to hide inline suggestions while keeping Ctrl+R history search, or change `editor.autosuggestion_format` using `$text` and the prompt styling tags. Use `editor.autosuggestion_metadata_format` to style inline metadata separately. `completion.enabled = false` disables the completion menu independently.

Press Ctrl+G again or Esc to leave mouse editing and restore native scrolling and selection. Enter also leaves mouse editing, including when continuing an incomplete command.

With `$path` visible, press Alt+G (Option+G on macOS) to focus its rightmost segment, Tab or Shift-Tab to select a segment, and Enter to change directory while preserving the command being edited. Escape cancels selection.

## How it works

Sopra runs as the editor behind Zsh's ZLE integration rather than as a replacement shell. Each interactive shell starts one private Rust worker and keeps it alive across commands. The Zsh bridge sends the current buffer, cursor, working directory, and environment snapshot over a private Unix socket; Sopra returns the edited buffer and the action Zsh should perform. The shell-native `sopra --help` command shows the configuration path. Zsh then accepts the command and executes it using the existing shell environment.

The shared `sopra-shell-context` crate owns environment snapshots and command lookup; prompts, widgets, syntax highlighting, and completion consume it independently. See [the architecture reference](docs/architecture.md) for ownership and extension boundaries.

The worker acquires the terminal only while editing and restores it before returning control to Zsh, so foreground programs receive terminal input normally. The worker retains configuration, history state, completion graphs, the syntax checker, the terminal palette, and widget instances across edits; shell and completion context refresh on every request without changing the worker’s process environment. The bridge stops the worker on shell exit, and the worker monitors its owning shell.

This keeps shell behavior in Zsh: aliases, functions, variables, command lookup, and command execution continue to use the current interactive shell.

The completion parser's ownership, source flow, and command-agnostic design rules are documented in [docs/completions.md](docs/completions.md).

## Development

Developer commands use a discovered checkout or a saved link:

```sh
sopra dev link /path/to/sopra
sopra dev check
sopra dev test syntax:: -- --nocapture
sopra dev shell
```

The link is saved beside config.toml in `dev-checkout` and affects developer commands only. Checkout selection prefers the link, then `SOPRA_REPO_ROOT`, then ancestors of the current directory and executable. `build` builds the debug worker; `check` runs formatting and Clippy; `test` runs workspace tests and forwards Cargo arguments; `shell` builds and starts an isolated Zsh. Use `sopra dev path` to inspect the target and `sopra dev unlink` to remove the link. For the first build, use `./scripts/start.sh`.

Inspect syntax without executing it, or inspect discovered completion graphs:

```sh
sopra debug syntax 'echo "$HOME" | cat'
sopra debug completions --root npm
sopra debug completions --chunk npm access
```

Run shell syntax checks and `python3 scripts/terminal-harness.py` alongside `sopra dev check` and `sopra dev test` before opening a pull request. See [terminal sessions](docs/terminal-sessions.md) for persistent interactive testing and `sopra dev --help` / `sopra debug --help` for command details.

## Contributing

Keep changes focused on editor, Zsh integration, or completion behavior, and add a focused test for behavior changes. Run the full check sequence above before opening a pull request. Bug reports are most useful when they include the operating system, Zsh version, terminal, installation method, and a minimal reproduction.

## License

Sopra is licensed under the [GNU General Public License v3.0](./LICENSE).
