# Interactive terminal sessions for agents

Use `scripts/terminal-session.py` when an agent needs to drive one interactive
program through several turns and inspect what a terminal actually renders.
The session owns a child process, a PTY, and a virtual character screen for its
entire lifetime, so later input sees the shell state and screen left by earlier
input.

This is deliberately separate from `scripts/terminal-harness.py`. The harness
is a CI smoke test for PTY behavior and raw protocol output; the session tool is
an agent-facing interaction boundary for keystrokes, repainting, resizing, and
screen assertions.

## Quick start

Run the controller from the repository root and keep its process handle alive:

```sh
python3 scripts/terminal-session.py --rows 24 --columns 80 -- zsh -f
```

The command after `--` is started once. Send one newline-terminated JSON object
per turn on the controller's standard input, then read exactly one JSON response
line. The controller keeps child output out of the JSON protocol and returns the
rendered screen by default; raw PTY bytes are opt-in for ANSI debugging.

For Sopra, start the same kind of persistent session with:

```sh
python3 scripts/terminal-session.py --rows 24 --columns 80 -- ./scripts/start.sh
```

A complete two-turn interaction looks like this:

```json
{"op":"send","data":"printf 'first turn\\n'","settle":0.1}
{"op":"key","key":"ENTER"}
{"op":"send","data":"printf 'second turn\\n'"}
{"op":"key","key":"ENTER"}
{"op":"read","timeout":0.2}
{"op":"screen"}
{"op":"close"}
```

Each JSON object above must end with a newline when sent to the controller. A
`send` operation types text but does not press Enter; submit the line explicitly
with `{"op":"key","key":"ENTER"}`. Keep the controller's process handle
until the final `close` response, because starting a new controller loses the
child's environment, history, and screen.

Responses are screen-first: they omit raw output unless the request includes
`"include_output": true`. That keeps normal turns bounded by the configured
screen size instead of expanding with every ANSI repaint.

## Protocol operations

| Operation | Required fields | Behavior |
| --- | --- | --- |
| `send` | `data` | Writes UTF-8 text to the PTY, then reads for `settle` seconds. It does not submit the current line. |
| `paste` | `data` | Sends bracketed-paste markers around the text, then reads for `settle` seconds. |
| `key` | `key` | Sends a named key or one literal character, then reads for `settle` seconds. |
| `read` | — | Drains PTY output for `timeout` seconds without sending input. Use this after delayed repaint or command output. |
| `wait_for` | `text` | Keeps reading until `text` appears anywhere in the rendered screen or the timeout expires. The response contains the updated screen. |
| `resize` | `rows`, `columns` | Changes both the PTY window size and the virtual screen dimensions. Read afterward if the program repaints asynchronously. |
| `screen` | — | Returns the current screen without reading the PTY. |
| `close` | — | Terminates the child, returns the final snapshot, and ends the controller. |

`settle` defaults to `0.05` seconds, `read.timeout` defaults to `0.05`, and
`wait_for.timeout` defaults to `3.0`. These are quiet-period or polling limits,
not guarantees that a command has finished; use a longer `read` or a specific
screen assertion when the program can respond slowly.

All operations accept the optional boolean `include_output`. Set it to `true`
only when that operation's raw PTY bytes are needed for protocol debugging; it
does not change what the virtual screen receives or return bytes accumulated by
earlier operations.

The `key` operation accepts `ENTER`, `RETURN`, `TAB`, `ESC`, `BACKSPACE`,
`DELETE`, `UP`, `DOWN`, `LEFT`, `RIGHT`, `HOME`, `END`, `PAGEUP`, `PAGEDOWN`,
`INSERT`, `CTRL-C`, `CTRL-D`, `CTRL-Z`, `CTRL-L`, `CTRL-A`, `CTRL-E`, `CTRL-U`,
`CTRL-K`, and `CTRL-W`, plus one-character literal keys. A JSON `CTRL-C` sends
byte `0x03` to the child; pressing Ctrl-C in the outer terminal interrupts the
controller process instead.

## Reading responses and asserting the screen

Every successful response has this shape:

```json
{
  "ok": true,
  "op": "read",
  "screen": {
    "rows": 24,
    "columns": 80,
    "cursor": {"row": 2, "column": 5},
    "cursor_visible": true,
    "lines": ["..."],
    "alive": true
  }
}
```

`screen.lines` is the rendered character grid with trailing spaces removed from
each line. Cursor coordinates are zero-based. Normal responses contain no raw
PTY data, so their size is bounded by the screen dimensions. When a request
sets `include_output` to `true`, the response also contains `output_base64`, the
raw bytes read during that operation encoded so ANSI escape bytes cannot break
the JSON stream. Decode that field only when debugging terminal protocol
behavior; use `screen.lines` for normal text and layout assertions.

For example, this asks for raw bytes for one read without enabling them for the
rest of the session:

```json
{"op":"read","timeout":0.1,"include_output":true}
```

The virtual screen handles the cursor movement, erasing, scrolling, alternate
screen, SGR, palette-query, and common terminal status sequences needed by
text-oriented TUIs. It models a character grid rather than pixels, colors,
graphics protocols, mouse reporting, font shaping, or every terminal
extension, so use raw output or a real terminal when those details are the
behavior under test.

## Reliable interaction pattern

1. **Choose dimensions deliberately.** Use the same `rows` and `columns` as the
   scenario you want to reproduce, because wrapping and completion-menu layout
   depend on the PTY size.
2. **Drain startup output.** Send `read` with a short timeout or wait for a
   stable prompt before typing. A program can repaint after its first prompt,
   so do not assume the first response is the complete startup screen.
3. **Separate typing from submission.** Use `send` for text and `key` with
   `ENTER` for submission. This makes line-editor behavior, completion, and
   cursor movement observable at the same granularity as a user interaction.
4. **Wait for the event you care about.** Use `read` after asynchronous output or
   `resize`; use `wait_for` for a stable marker. `wait_for` searches the entire
   rendered screen, including typed command echo, so `wait_for("access")` can
   succeed before an interactive completion result is actually accepted.
5. **Assert the rendered state.** Check `screen.lines`, the cursor, and
   `alive`. For interactive editors, assert both the typed line and the menu or
   prompt that proves the intended state was reached instead of asserting only
   that input was echoed.
6. **Close every session.** Send the `close` JSON request in the controller or
   use the Python context manager. If the scenario needs a normal child exit,
   send `CTRL-C` or `CTRL-D` first; `close` remains the cleanup boundary when
   the child is intentionally left running or is stuck.

## Example: testing Sopra completion

The following requests type `npm access` without accepting the completion:

```json
{"op":"send","data":"npm access","settle":0.2}
{"op":"screen"}
```

The screen should contain the typed command and a completion menu containing
`access` and `Tab to accept`. Exact prompt text, border width, and timing text
can vary with the working directory and terminal width, so assert those stable
markers and the relevant line geometry rather than comparing the entire ANSI
byte stream. To accept the completion, send `{"op":"key","key":"TAB"}`;
to submit the resulting command, send `ENTER` separately.

## Python API

Tests that need direct access can use the same session model without the
JSON-lines controller:

```python
from pathlib import Path

from scripts.terminal_session import TerminalSession

root = Path.cwd()
with TerminalSession(
    (str(root / "scripts" / "start.sh"),),
    cwd=str(root),
    rows=24,
    columns=80,
) as terminal:
    terminal.read(0.5)
    terminal.send_text("npm access")
    terminal.read(1.0)
    assert "access" in terminal.screen.text(trim=False)
    terminal.key("CTRL-C")
```

`TerminalSession.screen.text(trim=False)` preserves the full rectangular grid,
including trailing spaces, which is useful for exact cursor and layout checks.
`terminal.output` accumulates all raw PTY bytes seen by the session. The
`wait_for` method uses the same screen-wide search as the controller and has
the same typed-echo caveat.

## Choosing this tool versus the harness

Use `scripts/terminal-harness.py` for a fast, deterministic CI check that a
known PTY scene emits the expected protocol bytes. Use this session tool for an
agent test that needs persistent shell state, multiple turns, special keys,
resizing, delayed reads, or assertions about what a human would see on screen.
Keep both tests when a change affects both the low-level protocol contract and
the interactive behavior built on top of it.

Run the focused session smoke test with:

```sh
python3 scripts/terminal-session-test.py
```

The session tool uses only the Python standard library and is not part of the
normal CI harness path.
