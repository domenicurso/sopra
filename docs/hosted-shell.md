# Hosted shells and scrollback

Run `sopra shell` to start an interactive Zsh inside a Sopra-owned PTY. It loads
your `.zshenv` and `.zshrc`, then enables the Sopra bridge if it isn't already
loaded. It inherits your working directory. `scripts/start.sh` runs this mode
with the repository's isolated development configuration.

The existing bridge-only setup still works. Hosted mode is required for ordinary
wheel scrolling together with mouse selection: the outer Sopra process receives
mouse events, scrolls its own history, and routes command-line clicks to the
inner editor. It does not require Shift+scroll or changes to Ghostty.

## Browsing and interacting

Scroll the wheel, press Page Up, or press Alt+H to open session history. The live
command line stays below it, so you can browse output without losing your draft.
Esc returns to the live terminal. Alt+/ opens search directly.

Each executed command has a block with its command, output, exit status, and
elapsed time. Click a header to select and collapse or expand the block; Tab and
Shift+Tab select adjacent blocks. Click the header's copy/run buttons or the
footer controls, or use these keys while browsing:

| Key | Action |
| --- | --- |
| Up/Down, Page Up/Down, wheel | Scroll history. |
| `/`, Enter | Enter a search and jump to a match. |
| `n`, `N` | Next or previous match, wrapping at the end. |
| `c`, Ctrl+C | Copy the selection, or the selected block if nothing is selected. |
| `r` | Rerun the selected command in the current shell directory. |
| `e` | Replace the current input with the selected command for editing. |
| `f`, Space | Collapse or expand the selected block. |
| `o` | Open the first existing file path printed by the selected block. |
| Esc | Leave search entry, then leave history. |

Drag over history text to select it, double-click a word, or triple-click a
logical line. Selection follows logical text when the window width changes.
Searching also expands collapsed blocks that contain a match.
Ctrl+click a printed file path to open that file relative to the command's
original directory. Clipboard copying uses the terminal's OSC 52 support.

Rerun requires an idle primary prompt. The command is checked and submitted
through the editor; any draft is restored afterward. Reuse (`e`) deliberately
replaces the draft. Rerun and reuse preserve multiline command text. Other text
keys are sent to the pinned live editor; history action keys retain their meaning
until Esc returns to live mode.

## Retention and terminal behavior

```toml
[session]
max_blocks = 500
scrollback_lines = 10000
max_output_bytes = 8388608
```

These positive integer limits apply when a hosted shell starts. Old blocks are
evicted by count or total retained text size. Each running command also has a
capture byte limit and terminal scrollback line limit; hitting the byte limit
adds a truncation notice. Output history is held in memory for this hosted
session. Zsh's persisted command history remains separate.

Foreground programs receive keyboard input and PTY signals normally. When a
program enters the alternate screen, mouse events go to that program when it
requests them. Alt+H remains available to inspect earlier commands. The host
restores terminal modes when it exits or receives SIGINT, SIGHUP, or SIGTERM,
and hangs up its shell and foreground jobs.

The host implements core xterm text rendering through a virtual terminal. It
preserves live ANSI colors and attributes, answers terminal geometry queries,
and forwards titles, palette queries, and clipboard writes. The history browser
shows plain captured text; progress updates are interpreted before capture.
Full-screen applications aren't recorded as screen recordings, and graphics
protocols and terminal-specific extensions are not supported. Startup output
before the first command is visible live but isn't a command block.
