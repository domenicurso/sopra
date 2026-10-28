#!/usr/bin/env python3
"""Exercise the editor against a terminal that reflows its primary screen."""
from __future__ import annotations

import base64
import json
import subprocess
import time
from itertools import chain
from pathlib import Path
from typing import TypedDict, cast

from terminal_session import TerminalSession
from terminal_config import terminal_config


class Snapshot(TypedDict):
    responses: list[str]
    lines: list[str]
    base: int
    cursor: list[int]


class ReflowScreen:
    def __init__(self, process: subprocess.Popen[str]) -> None:
        self.process = process
        self.rows, self.columns = 12, 120

    def request(self, payload: dict[str, object]) -> Snapshot:
        assert self.process.stdin is not None and self.process.stdout is not None
        self.process.stdin.write(json.dumps(payload) + "\n")
        self.process.stdin.flush()
        return cast(Snapshot, json.loads(self.process.stdout.readline()))

    def feed(self, data: bytes) -> tuple[bytes, ...]:
        result = self.request({"op": "feed", "data": base64.b64encode(data).decode()})
        return tuple(base64.b64decode(response) for response in result["responses"])

    def resize(self, rows: int, columns: int) -> None:
        self.rows, self.columns = rows, columns
        self.request({"op": "resize", "rows": rows, "columns": columns})

    def snapshot(self) -> Snapshot:
        return self.request({"op": "snapshot"})

    def text(self, trim: bool = True) -> str:
        snapshot = self.snapshot()
        return "\n".join(snapshot["lines"][snapshot["base"] : snapshot["base"] + self.rows])


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
    assert snapshot["cursor"][0] == len("test $ " + command), snapshot


def sweep(terminal: TerminalSession, screen: ReflowScreen, minimum: int) -> None:
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
    assert after["base"] == before["base"], (before, after)
    assert after["cursor"][1] == before["cursor"][1], (before, after)


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
        assert_prompt(screen)
        sweep(terminal, screen, 40)
        assert_prompt(screen)
        terminal.resize(12, 40)
        terminal.read(0.2)
        assert_history(screen)
        assert_prompt(screen)
        terminal.send_text("echo resize-marker")
        terminal.key("ENTER")
        terminal.read(0.3)
        assert "$ echo resize-marker" in screen.snapshot()["lines"], screen.snapshot()
        assert "resize-marker" in screen.snapshot()["lines"]
        assert_history(screen)
        assert_prompt(screen)
        terminal.resize(12, 120)
        terminal.read(0.2)
        assert_prompt(screen)
        command = "echo " + "x" * 60
        terminal.send_text(command)
        terminal.read(0.2)
        assert_prompt(screen, command)
        sweep(terminal, screen, 20)
        assert_prompt(screen, command)
        terminal.key("ENTER")
        terminal.read(0.3)
        lines = screen.snapshot()["lines"]
        assert "$ " + command in lines and "x" * 60 in lines, lines
        assert_history(screen)
        assert_prompt(screen)


def main() -> None:
    root = Path(__file__).resolve().parent.parent
    subprocess.run(["cargo", "build", "--quiet", "--locked", "--bin", "sopra-worker"], cwd=root, check=True)
    with terminal_config("[prompt]\nleft = 'test $ '\nright = 'rprompt'\n[editor]\ncompletion = false\ncursor_animation = false\nsyntax_highlighting = false\n") as config_root, subprocess.Popen(
        ["node", str(root / "scripts/terminal-reflow-screen.cjs")],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        text=True,
    ) as process:
        run_session(root, ReflowScreen(process), config_root)
    print("terminal reflow: ok (history, cursor, right prompt, transient prompt)")


if __name__ == "__main__":
    main()
