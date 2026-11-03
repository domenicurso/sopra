#!/usr/bin/env python3
"""Exercise the editor against a terminal that reflows its primary screen."""
from __future__ import annotations

import subprocess
import time
from itertools import chain
from pathlib import Path

from terminal_reflow import ReflowScreen
from terminal_session import TerminalSession
from terminal_config import terminal_config



def assert_history(screen: ReflowScreen) -> None:
    lines = screen.snapshot()["lines"]
    for index in range(1, 16):
        assert lines.count(f"history-{index}") == 1, lines


def assert_prompt(screen: ReflowScreen, command: str = "") -> None:
    snapshot = screen.snapshot()
    matches = [(row, line) for row, line in enumerate(snapshot["lines"]) if line.startswith("test $ ")]
    assert len(matches) == 1, snapshot
    row, line = matches[0]
    assert line.startswith("test $ " + command), line
    assert line.endswith("rprompt") and len(line) == screen.columns, line
    assert row - snapshot["base"] == snapshot["cursor"][1], snapshot
    # Check the painted cursor as well as the terminal cursor used for recovery.
    assert snapshot["cursor"][0] == len("test $ " + command), snapshot
    assert snapshot["paintedCursors"] == [[len("test $ " + command), row]], snapshot


def wait_prompt(terminal: TerminalSession, screen: ReflowScreen, command: str = "") -> None:
    # Resizing can return before the worker answers the cursor query and repaints.
    deadline = time.monotonic() + 3
    while True:
        terminal.read(0.05)
        try:
            assert_prompt(screen, command)
            return
        except AssertionError:
            if time.monotonic() >= deadline:
                raise


def sweep(terminal: TerminalSession, screen: ReflowScreen, minimum: int, command: str = "") -> None:
    before = screen.snapshot()
    for width in chain(range(119, minimum - 1, -1), range(minimum + 1, 121)):
        terminal.resize(12, width)
        terminal.read(0)
        time.sleep(0.001)
        if width % 10 == 0:
            assert_history(screen)
    terminal.read(0.2)
    assert_history(screen)
    after = screen.snapshot()
    # A wrapped command at the bottom needs additional viewport rows. The
    # terminal may keep those rows after expansion, but the prompt must stay
    # attached to the same scrollback content rather than leave duplicate rows.
    extra_rows = len("test $ " + command) // minimum
    assert before["base"] <= after["base"] <= before["base"] + extra_rows, (before, after)
    assert after["base"] + after["cursor"][1] == before["base"] + before["cursor"][1], (before, after)


def run_session(root: Path, screen: ReflowScreen, config_root: str) -> None:
    environment = {
        "SOPRA_REPO_ROOT": str(root),
        "SOPRA_BIN": str(root / "target/debug/sopra-worker"),
        "ZDOTDIR": str(root / "examples/isolated"),
        "XDG_CONFIG_HOME": config_root,
    }
    with TerminalSession(("zsh", "-di"), rows=12, columns=120, cwd=str(root), env=environment) as terminal:
        terminal.screen = screen
        terminal.wait_for("rprompt", timeout=20)
        terminal.send_text("for n in {1..15}; do echo history-$n; done")
        terminal.key("ENTER")
        terminal.wait_for("history-15", timeout=10)
        terminal.read(0.3)
        assert_history(screen)
        wait_prompt(terminal, screen)
        sweep(terminal, screen, 40)
        wait_prompt(terminal, screen)
        terminal.resize(12, 40)
        terminal.read(0.2)
        assert_history(screen)
        wait_prompt(terminal, screen)
        terminal.send_text("echo resize-marker")
        terminal.key("ENTER")
        terminal.read(0.3)
        assert "$ echo resize-marker" in screen.snapshot()["lines"], screen.snapshot()
        assert "resize-marker" in screen.snapshot()["lines"]
        assert_history(screen)
        wait_prompt(terminal, screen)
        terminal.resize(12, 120)
        terminal.read(0.2)
        wait_prompt(terminal, screen)
        command = "echo " + "x" * 60
        terminal.send_text(command)
        terminal.read(0.2)
        wait_prompt(terminal, screen, command)
        sweep(terminal, screen, 20, command)
        wait_prompt(terminal, screen, command)
        terminal.key("ENTER")
        terminal.read(0.3)
        lines = screen.snapshot()["lines"]
        assert "$ " + command in lines and "x" * 60 in lines, lines
        assert_history(screen)
        wait_prompt(terminal, screen)


def main() -> None:
    root = Path(__file__).resolve().parent.parent
    subprocess.run(["cargo", "build", "--quiet", "--locked", "--bin", "sopra-worker"], cwd=root, check=True)
    with terminal_config("[prompt]\nleft = 'test $ '\nright = 'rprompt'\n[completion]\nenabled = false\n[editor]\ncursor_animation = false\nsyntax_highlighting = false\n") as config_root, subprocess.Popen(
        ["node", str(root / "scripts/terminal-reflow-screen.cjs")],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        text=True,
    ) as process:
        run_session(root, ReflowScreen(process), config_root)
    print("terminal reflow: ok (history, cursor, right prompt, transient prompt)")


if __name__ == "__main__":
    main()
