#!/usr/bin/env python3
"""Hosted scrollback, block actions, draft retention, and rapid resize tests."""
from __future__ import annotations

import base64
import re
from pathlib import Path
from terminal_session import TerminalSession

from terminal_host import browse, hosted, submit, wait_until


def test_blocks() -> None:
    with hosted() as (terminal, root):
        submit(terminal, "printf 'alpha\\nbeta\\n'; false")
        terminal.send_text("echo unfinished draft")
        terminal.read(0.2)
        browse(terminal)
        screen = terminal.screen.text()
        assert "exit 1" in screen and "alpha" in screen and "beta" in screen, screen
        assert "echo unfinished draft" in screen.splitlines()[-1], screen
        terminal.send(b"c")
        terminal.read(0.2)
        copies = re.findall(rb"\x1b]52;c;([^\x1b\x07]*)", terminal.output)
        assert copies and b"alpha\nbeta" in base64.b64decode(copies[-1])
        row = next(index for index, line in enumerate(terminal.screen.lines()) if line.strip() == "alpha") + 1
        for _ in range(2):
            terminal.send(f"\x1b[<0;4;{row}M\x1b[<0;4;{row}m")
        terminal.send(b"c")
        terminal.read(0.1)
        copies = re.findall(rb"\x1b]52;c;([^\x1b\x07]*)", terminal.output)
        assert base64.b64decode(copies[-1]) == b"alpha"
        terminal.send(b"f")
        terminal.read(0.1)
        assert not any(line.strip() == "alpha" for line in terminal.screen.lines())
        terminal.send(b"f")
        terminal.read(0.1)
        terminal.send(b"/")
        terminal.read(0.05)
        terminal.send_text("beta")
        terminal.key("ENTER")
        terminal.read(0.1)
        assert "2 matches" in terminal.screen.text(), terminal.screen.text()
        resize_rerun_and_exit(terminal, root)

def resize_rerun_and_exit(terminal: TerminalSession, root: Path) -> None:
    for width in list(range(120, 39, -2)) + list(range(40, 121, 2)):
        terminal.resize(24, width)
        terminal.read(0.002)
    terminal.read(0.3)
    assert "echo unfinished draft" in terminal.screen.text()
    prompt = next(line for line in terminal.screen.lines() if "host" in line and "rprompt" in line)
    assert prompt.endswith("rprompt") and len(prompt) == 120, prompt
    terminal.send(b"r")
    terminal.read(0.3)
    terminal.wait_for("echo unfinished draft", timeout=5)
    assert "Session history" not in terminal.screen.text()
    assert "echo unfinished draft" in terminal.screen.text()
    browse(terminal)
    assert "#2" in terminal.screen.text(), terminal.screen.text()
    terminal.key("ESC")
    terminal.read(0.1)
    terminal.key("CTRL-U")
    submit(terminal, "printf 'README.txt\\n'")
    (root / "README.txt").write_text("open this")
    browse(terminal)
    terminal.send(b"o")
    wait_until(terminal, lambda: (root / "opened").exists() and (root / "opened").read_text() == str((root / "README.txt").resolve()))
    (root / "opened").unlink()
    row = next(index for index, line in enumerate(terminal.screen.lines()) if line.strip() == "README.txt") + 1
    terminal.send(f"\x1b[<16;4;{row}M")
    wait_until(terminal, lambda: (root / "opened").exists())
    terminal.send(b"e")
    terminal.read(0.2)
    assert "printf 'README.txt\\n'" in terminal.screen.text()
    terminal.key("CTRL-U")
    submit(terminal, "exit 7")
    wait_until(terminal, lambda: not terminal.alive)
    assert terminal.returncode == 7, terminal.returncode
    assert b"\x1b[?1049l" in terminal.output

if __name__ == "__main__":
    test_blocks()
    print("Hosted blocks, scrollback, search, rerun, draft retention, and resize passed")
