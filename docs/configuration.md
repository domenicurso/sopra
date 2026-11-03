# Configuration and prompts

Configure Sopra in `~/.config/sopra/config.toml` (or `$XDG_CONFIG_HOME/sopra/config.toml`). Edit this file to change the prompt and editor options.

```toml
[editor]
cursor_animation = true
syntax_highlighting = true
autosuggestions = true

[completion]
enabled = true

[prompt]
left = "[red]$user[/red] [dim]in[/dim] [bold blue]$path[/bold blue]\n[dim]$[/dim] "
right = "[red]$status[/red]  [dim]$duration[/dim] [cyan]$git[/cyan]"
transient = "[dim]$[/dim] "
secondary = "[dim]>[/dim] "

[widgets.clock]
format = "%H:%M:%S"
```

`config.toml` is the only configuration file. Run `sopra refresh` after editing it,
or use `sopra config edit` to open, validate, and refresh it in one step.

## Prompt templates

Templates use `$widget` references and matching style tags, such as
`[bold blue]$path[/bold blue]`. Tags can nest. Styles include colors, `bold`,
`dim`, `italic`, `underline`, and `reverse`. Use `$$` for a literal dollar sign
before a name, and `[[` for a literal opening bracket. A standalone `$` is literal.

Text between widget references is a connector: it appears only between nonempty
values. In the right prompt above, a successful status disappears without leaving
leading whitespace; if only Git is available, there are no separators. Missing middle
values leave one connector, chosen from the text immediately after the preceding
visible value. Text before the first reference and after the last reference stays
literal, so prompt characters and final input spacing are preserved.

## Built-in widgets

| Widget | Display |
| --- | --- |
| `path` | Current directory, abbreviated relative to your home directory. |
| `git` | Git branch and counts of changed/untracked files and commits ahead/behind; empty outside a repository. |
| `git_status` | The Git counts without the branch. |
| `clock` | Local time, refreshed while editing. |
| `status` | Exit status of the last command when it failed. |
| `duration` | Duration of the last command. |
| `user`, `host`, `shell` | Current user, host, and shell name. |

## Multiline prompts

Use `\n` to start another prompt row. The right prompt stays on the first row,
and the command starts after the last row. Incomplete input stays in the editor,
with `secondary` on each continuation line; the primary prompt becomes transient
only when the entire command is submitted. Aliases that introduce unfinished syntax use the live shell's continuation handling.

Shift+Enter inserts a newline at the cursor without submitting, using the configured secondary prompt. This requires a terminal that reports modified Enter keys, such as Ghostty. Enter still submits complete commands and continues incomplete syntax.

Widgets inherit the color surrounding their reference. They may dim punctuation,
emphasize text, or reverse a selection, but they don't select their own colors.
`widgets.*` controls content and behavior: the clock's `format` uses `strftime`,
and Git accepts `label`, `changed`, `untracked`, `ahead`, `behind`, `open`, `close`,
and `separator`. Put Rust widgets in `~/.config/sopra/widgets/`; their filenames
become references automatically. See [the widget SDK](widgets.md).

See [editing and history](usage.md) for autosuggestion settings and key bindings.
