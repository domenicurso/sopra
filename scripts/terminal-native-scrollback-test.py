#!/usr/bin/env python3
"""Keep output in native terminal history while mouse editing is opt-in."""
from __future__ import annotations

import subprocess
import time
from pathlib import Path

from terminal_config import terminal_config
from terminal_reflow import ReflowScreen
from terminal_session import TerminalSession

ROOT = Path(__file__).resolve().parent.parent
CONFIG = '''[prompt]
left = "native\\n$ "
right = "RIGHT"
[editor]
cursor_animation = false
autosuggestions = false
[completion]
enabled = false
'''


def assert_mode(screen: ReflowScreen, enabled: bool) -> None:
    snapshot = screen.snapshot()
    assert snapshot["buffer"] == "normal", snapshot
    assert snapshot["mouseTracking"] == ("drag" if enabled else "none"), snapshot
    assert ("Mouse editing · Esc to exit" in screen.text()) == enabled, snapshot


def exercise(screen: ReflowScreen, config_root: str) -> None:
    env = {"SOPRA_REPO_ROOT": str(ROOT), "SOPRA_BIN": str(ROOT / "target/debug/sopra-worker"),
           "ZDOTDIR": str(ROOT / "examples/isolated"), "XDG_CONFIG_HOME": config_root}
    with TerminalSession(("zsh", "-di"), rows=12, columns=120, cwd=str(ROOT), env=env) as terminal:
        terminal.screen = screen
        terminal.wait_for("RIGHT", timeout=20)
        assert_mode(screen, False)
        terminal.send_text("for i in {1..60}; do print native-history-$i; done")
        terminal.key("ENTER")
        terminal.wait_for("native-history-60", timeout=10)
        terminal.read(0.3)
        snapshot = screen.snapshot()
        assert snapshot["base"] > 0, snapshot
        assert "native-history-1" in snapshot["lines"][:snapshot["base"]], snapshot
        assert "native-history-1" not in screen.text().splitlines(), snapshot
        screen.request({"op": "scroll", "lines": -40})
        viewport = screen.snapshot()["viewport"]
        assert viewport < snapshot["base"], snapshot
        terminal.read(0.2)
        assert screen.snapshot()["viewport"] == viewport
        screen.request({"op": "scroll", "lines": 1000})
        exercise_mode(terminal, screen)
        assert "native-history-1" in screen.snapshot()["lines"]
        terminal.send(b"\x07")
        terminal.read(0.1)
        assert_mode(screen, True)
        terminal.key("CTRL-D")
        deadline = time.monotonic() + 5
        while terminal.alive and time.monotonic() < deadline:
            terminal.read(0.05)
        assert not terminal.alive, screen.text()
        assert screen.snapshot()["mouseTracking"] == "none"
        assert b"\x1b[?1049h" not in terminal.output


def exercise_mode(terminal: TerminalSession, screen: ReflowScreen) -> None:
    terminal.send_text("echo draft")
    terminal.send(b"\x07")
    terminal.read(0.2)
    assert_mode(screen, True)
    terminal.key("ESC")
    terminal.read(0.2)
    assert_mode(screen, False)
    assert "$ echo draft" in screen.text()
    assert "RIGHT" in screen.text()
    terminal.send(b"\x07\x07")
    terminal.read(0.2)
    assert_mode(screen, False)
    terminal.send(b"\x07")
    terminal.read(0.1)
    terminal.key("ENTER")
    terminal.read(0.4)
    assert_mode(screen, False)
    assert "draft" in screen.text().splitlines()
    terminal.send(b"\x07")
    terminal.read(0.1)
    terminal.key("CTRL-C")
    terminal.read(0.3)
    assert_mode(screen, False)
    terminal.send_text("echo \\")
    terminal.send(b"\x07")
    terminal.read(0.1)
    terminal.key("ENTER")
    terminal.read(0.4)
    assert_mode(screen, False)
    terminal.key("CTRL-C")
    terminal.read(0.3)
    terminal.paste("printf 'literal-one\\n'\nprintf 'literal-two\\n'")
    terminal.read(0.2)
    assert "literal-two" not in screen.text().splitlines()
    terminal.key("ENTER")
    terminal.read(0.4)
    assert "literal-one" in screen.text().splitlines()
    assert "literal-two" in screen.text().splitlines()


def main() -> None:
    subprocess.run(["cargo", "build", "--quiet", "--locked"], cwd=ROOT, check=True)
    with terminal_config(CONFIG) as config_root, subprocess.Popen(
        ["node", str(ROOT / "scripts/terminal-reflow-screen.cjs")],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True,
    ) as process:
        exercise(ReflowScreen(process), config_root)
    print("Native scrollback and opt-in mouse editing: ok")


if __name__ == "__main__":
    main()
