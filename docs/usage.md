# Editing and commands

## Command-line editing

Enter submits complete commands and continues incomplete syntax. Shift+Enter
inserts a newline at the cursor without submitting; this requires a terminal that
reports modified Enter keys, such as Ghostty. Continuation lines use the configured
[secondary prompt](configuration.md).

## History

History autosuggestions show the rest of your most recent matching command as dim text. Up and Down cycle the matches while the completion menu is closed; when it is open, they move through its contextual completions. Full commands from history never appear in that menu; history only breaks ties between equally strong valid completions. Alt+Up and Alt+Down cycle history matches in either view. Right or End accepts the active history suggestion; Alt+Right accepts the next word. Escape dismisses the hint until you edit the command. Hints are clipped to the current screen row and never enter the command buffer until accepted.

Ctrl+R opens a separate history search, seeded with the current command. Type to filter by substring, use Up/Down or Ctrl+R to cycle matches, and Enter or Tab to copy a result into the command line without executing it. Escape or Ctrl+C cancels and restores the draft and cursor. Search works independently of completion and autosuggestion settings.

History search rows show their age beside the command when Zsh has a timestamp, or “History” when the time is unknown. Ages under a minute show seconds, such as `12s ago`. The inline suggestion shows the same metadata in dim italics after its text when there is room; metadata is never inserted into the command. Without a matching inline suggestion or focused completion, Up and Down recall command history in place; Down restores your draft after returning past the newest entry. Multiline cursor movement takes priority within an existing command.

Sopra reads the active Zsh history, including commands loaded from your history file and commands run in this session. It does not maintain a separate history file. If `HISTFILE` is unset, the bridge uses `${ZDOTDIR:-$HOME}/.zsh_history` and defaults to saving 10,000 entries. Existing history paths and limits, including an explicit `SAVEHIST=0`, are preserved. Commands are appended during the session, with timestamps; an existing `SHARE_HISTORY` setup keeps its live sharing behavior.

Set `editor.autosuggestions = false` to hide inline suggestions while keeping Ctrl+R history search, or change `editor.autosuggestion_format` using `$text` and the prompt styling tags. Use `editor.autosuggestion_metadata_format` to style inline metadata separately. `completion.enabled = false` disables the completion menu independently.

## Mouse editing and prompt navigation

Press Ctrl+G to toggle command-line mouse editing, then drag to select, double-click a word, or triple-click the whole line. Esc returns to native terminal selection and scrolling. Ctrl-C copies selected text when the terminal supports clipboard writes; without a selection it interrupts the command.

Press Ctrl+G again or Esc to leave mouse editing and restore native scrolling and selection. Enter also leaves mouse editing, including when continuing an incomplete command.

With `$path` visible, press Alt+G (Option+G on macOS) to focus its rightmost segment, Tab or Shift-Tab to select a segment, and Enter to change directory while preserving the command being edited. Escape cancels selection.

## Session commands

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
| `sopra cache status` | Show completion source and graph cache locations, file counts, and byte sizes. |
| `sopra cache clear [command]` | Delete saved completion data for all commands or one executable name, then reset the current worker's completion memory. |

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

For cache scope, invalidation, and other shells, see [cache lifecycle](completions.md#cache-lifecycle-and-presentation).

Help and widget listings adapt to the terminal width, and `config show` highlights
TOML. Redirected output stays plain; `NO_COLOR` also disables terminal styling.
