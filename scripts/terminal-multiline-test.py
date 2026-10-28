#!/usr/bin/env python3
"""Exercise multi-line prompts and path focus with terminal reflow."""
from __future__ import annotations

import importlib.util
import subprocess
import tempfile
import time
from itertools import chain
from pathlib import Path

from terminal_session import TerminalSession
from terminal_config import terminal_config

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("reflow_test", ROOT / "scripts/terminal-reflow-test.py")
assert spec and spec.loader
reflow = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reflow)


def exercise(screen: reflow.ReflowScreen, directory: Path, config_root: str) -> None:
    env = {
        "SOPRA_REPO_ROOT": str(ROOT), "SOPRA_BIN": str(ROOT / "target/debug/sopra-worker"),
        "ZDOTDIR": str(ROOT / "examples/isolated"), "XDG_CONFIG_HOME": config_root,
    }
    with TerminalSession(("zsh", "-di"), rows=12, columns=120, cwd=str(directory), env=env) as terminal:
        terminal.screen = screen
        terminal.wait_for("RIGHT", timeout=20)
        terminal.read(0.3)
        terminal.send_text("for n in {1..15}; do echo history-$n; done")
        terminal.key("ENTER")
        terminal.wait_for("history-15", timeout=10)
        terminal.read(0.3)
        before = screen.snapshot()
        for number in range(1, 16):
            assert before["lines"].count(f"history-{number}") == 1, before
        lines = before["lines"]
        path_rows = [i for i, line in enumerate(lines) if str(directory) in line and line.endswith("RIGHT")]
        assert len(path_rows) == 1, before
        row = path_rows[0]
        assert lines[row + 1].rstrip() == "$", before
        terminal.send_text("echo kept")
        terminal.read(0.2)
        terminal.send(b"\x07")
        terminal.read(0.2)
        focused = screen.snapshot()
        assert focused["lines"][row].startswith(str(directory)), focused
        assert focused["lines"][row + 1].startswith("$ echo kept"), focused
        terminal.send(b"\t\x1b[Z")
        terminal.read(0.2)
        assert screen.snapshot()["lines"][row].startswith(str(directory))
        terminal.send(b"\x1b")
        terminal.read(0.2)
        exercise_resize(terminal, screen, directory)
        terminal.key("ENTER")
        terminal.read(0.6)
        state = screen.snapshot()
        assert "$ echo kept" in state["lines"] and "kept" in state["lines"], state
        assert sum(str(directory) in line for line in state["lines"]) == 1, state
        terminal.send_text("echo interrupted")
        terminal.key("CTRL-C")
        terminal.read(0.4)
        state = screen.snapshot()
        assert "$ echo interrupted" in state["lines"], state
        assert sum(str(directory) in line for line in state["lines"]) == 1, state


def exercise_resize(terminal: TerminalSession, screen: reflow.ReflowScreen, directory: Path) -> None:
    viewport_before = screen.snapshot()["base"]
    for width in chain(range(119, 39, -1), range(41, 121)):
        terminal.resize(12, width)
        terminal.read(0)
        time.sleep(0.001)
    terminal.read(0.4)
    state = screen.snapshot()
    assert state["base"] == viewport_before, state
    assert sum(str(directory) in line for line in state["lines"]) == 1, state
    for number in range(1, 16):
        assert state["lines"].count(f"history-{number}") == 1, state
    assert any(line.rstrip() == "$ echo kept" for line in state["lines"]), state
    for width in (40, 120):
        terminal.resize(12, width)
        terminal.read(0.3)
    state = screen.snapshot()
    assert state["base"] == viewport_before, state
    assert sum(str(directory) in line for line in state["lines"]) == 1, state


def exercise_secondary(config_root: str) -> None:
    env = {
        "SOPRA_REPO_ROOT": str(ROOT), "SOPRA_BIN": str(ROOT / "target/debug/sopra-worker"),
        "ZDOTDIR": str(ROOT / "examples/isolated"), "XDG_CONFIG_HOME": config_root,
    }
    with TerminalSession(("zsh", "-di"), rows=12, columns=60, cwd=str(ROOT), env=env) as terminal:
        terminal.wait_for("PRIMARY", timeout=20)
        terminal.send_text("for x in alpha beta; do")
        terminal.key("ENTER")
        terminal.wait_for("CONT >", timeout=5)
        terminal.send_text("echo $x")
        terminal.key("ENTER")
        terminal.read(0.4)
        lines = terminal.screen.text().splitlines()
        assert "CONT > echo $x" in lines, lines
        assert lines.count("CONT >") == 1, lines
        terminal.send_text("done")
        terminal.key("ENTER")
        terminal.read(0.5)
        lines = terminal.screen.text().splitlines()
        assert "CONT > done" in lines and "alpha" in lines and "beta" in lines, lines
        assert any(line.startswith("PRIMARY") for line in lines), lines


def exercise_wrapped_command(config_root: str) -> None:
    env = {
        "SOPRA_REPO_ROOT": str(ROOT), "SOPRA_BIN": str(ROOT / "target/debug/sopra-worker"),
        "ZDOTDIR": str(ROOT / "examples/isolated"), "XDG_CONFIG_HOME": config_root,
    }
    with TerminalSession(("zsh", "-di"), rows=12, columns=20, cwd=str(ROOT), env=env) as terminal:
        terminal.wait_for("P $", timeout=20)
        terminal.send_text("echo abcdefghijklmnopqrstuvwxyz")
        terminal.read(0.3)
        lines = terminal.screen.text().splitlines()
        assert "P $ echo abcdefghijk" in lines and "lmnopqrstuvwxyz" in lines, lines
        terminal.key("ENTER")
        terminal.read(0.5)
        lines = terminal.screen.text().splitlines()
        assert "$ echo abcdefghijklm" in lines and "nopqrstuvwxyz" in lines, lines
        assert "abcdefghijklmnopqrst" in lines and "uvwxyz" in lines, lines


def main() -> None:
    subprocess.run(["cargo", "build", "--quiet", "--locked", "--bin", "sopra-worker"], cwd=ROOT, check=True)
    with tempfile.TemporaryDirectory(prefix="sm-", dir="/tmp") as temporary:
        directory = Path(temporary).resolve() / "a" / "b"
        directory.mkdir(parents=True)
        with terminal_config("[prompt]\nleft = \"{path}\\n$ \"\nright = 'RIGHT'\ntransient = '$ '\n[editor]\ncompletion = false\ncursor_animation = false\nsyntax_highlighting = false\n") as config_root, subprocess.Popen(["node", str(ROOT / "scripts/terminal-reflow-screen.cjs")], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True) as process:
            exercise(reflow.ReflowScreen(process), directory, config_root)
    with terminal_config("[prompt]\nleft = 'PRIMARY $ '\nright = ''\ntransient = '$ '\nsecondary = 'CONT > '\n[editor]\ncompletion = false\ncursor_animation = false\n") as config_root:
        exercise_secondary(config_root)
    with terminal_config("[prompt]\nleft = 'P $ '\nright = ''\n[editor]\ncompletion = false\ncursor_animation = false\nsyntax_highlighting = false\n") as config_root:
        exercise_wrapped_command(config_root)
    print("multiline, wrapped, and secondary prompts: ok")


if __name__ == "__main__":
    main()
