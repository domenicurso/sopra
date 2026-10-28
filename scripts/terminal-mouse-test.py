#!/usr/bin/env python3
"""Exercise mouse selection in an interactive Sopra command line."""
from __future__ import annotations

import subprocess
from pathlib import Path

from terminal_config import terminal_config
from terminal_session import TerminalSession


def mouse(terminal: TerminalSession, row: int, column: int, code: int, end: str = "M") -> None:
    terminal.send(f"\x1b[<{code};{column + 1};{row + 1}{end}".encode())


def click(terminal: TerminalSession, row: int, column: int) -> None:
    mouse(terminal, row, column, 0)
    mouse(terminal, row, column, 0, "m")


def run(root: Path, config_root: str) -> None:
    env = {"XDG_CONFIG_HOME": config_root, "SOPRA_REPO_ROOT": str(root),
           "SOPRA_BIN": str(root / "target/debug/sopra-worker"),
           "ZDOTDIR": str(root / "examples/isolated")}
    with TerminalSession(("zsh", "-di"), rows=20, columns=80, cwd=str(root), env=env) as terminal:
        terminal.wait_for("test $ ", timeout=20)
        terminal.send_text("alpha beta gamma")
        terminal.read(0.3)
        row = next(index for index, line in enumerate(terminal.screen.lines()) if line.startswith("test $ alpha"))
        start = len("test $ ")
        mouse(terminal, row, start + 6, 0)
        mouse(terminal, row, start + 10, 32)
        mouse(terminal, row, start + 10, 0, "m")
        terminal.read(0.3)
        terminal.send(b"\x03")
        terminal.read(0.2)
        assert b"\x1b]52;c;YmV0YQ==\x1b\\" in terminal.output, bytes(terminal.output)
        terminal.send_text("X")
        terminal.read(0.3)
        assert terminal.screen.lines()[row].startswith("test $ alpha X gamma"), terminal.screen.lines()
        terminal.send(b"\x15")
        terminal.send_text("alpha beta gamma")
        terminal.read(0.3)
        click(terminal, row, start + 7)
        click(terminal, row, start + 7)
        terminal.read(0.3)
        terminal.resize(20, 40)
        terminal.read(0.3)
        terminal.resize(20, 80)
        terminal.read(0.3)
        terminal.send_text("Y")
        terminal.read(0.3)
        assert terminal.screen.lines()[row].startswith("test $ alpha Y gamma"), terminal.screen.lines()
        terminal.send(b"\x15")
        terminal.send_text("one two three")
        terminal.read(0.3)
        click(terminal, row, start + 1)
        click(terminal, row, start + 1)
        click(terminal, row, start + 1)
        terminal.read(0.3)
        terminal.send_text("done")
        terminal.read(0.3)
        assert terminal.screen.lines()[row].startswith("test $ done"), terminal.screen.lines()
        terminal.key("ENTER")
        terminal.read(0.4)
        assert b"\x1b[?1006h" in terminal.output and b"\x1b[?1006l" in terminal.output
        terminal.send_text("abcd")
        terminal.read(0.3)
        rows = [index for index, line in enumerate(terminal.screen.lines()) if line.startswith("test $ abcd")]
        assert rows and rows[-1] > row, terminal.screen.lines()
        click(terminal, rows[-1], start + 2)
        terminal.send_text("X")
        terminal.read(0.3)
        assert terminal.screen.lines()[rows[-1]].startswith("test $ abXcd"), terminal.screen.lines()


def main() -> None:
    root = Path(__file__).resolve().parent.parent
    subprocess.run(["cargo", "build", "--quiet", "--locked", "--bin", "sopra-worker"], cwd=root, check=True)
    config = "[prompt]\nleft = 'test $ '\nright = ''\n[editor]\ncompletion = false\ncursor_animation = false\n"
    with terminal_config(config) as config_root:
        run(root, config_root)
    print("mouse selection: ok (drag, double click, triple click, resize)")


if __name__ == "__main__":
    main()
