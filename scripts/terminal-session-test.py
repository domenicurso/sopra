#!/usr/bin/env python3
"""Focused smoke tests for the separate persistent terminal session tool."""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

from terminal_screen import VirtualTerminal
from terminal_session import TerminalSession


def test_screen_sequences() -> None:
    wrapped = VirtualTerminal(2, 8)
    wrapped.feed(b"12345678\x1b[1Gx")
    assert wrapped.lines()[0].startswith("x2345678")
    box = VirtualTerminal(2, 4)
    box.feed("───╮".encode())
    assert box.lines()[0] == "───╮"
    assert not box.lines()[1].strip()
    screen = VirtualTerminal(3, 8)
    assert screen.feed(b"one\x1b[2;1Htwo") == ()
    assert screen.lines()[:2] == ["one", "two"]
    assert screen.feed(b"\x1b[3;4H\x1b[6n") == (b"\x1b[3;4R",)
    assert screen.feed(b"\x1b]11;?\x1b\\") == (b"\x1b]11;rgb:0000/0000/0000\x1b\\",)
    assert screen.feed(b"\x1b]12;?\x1b\\") == (b"\x1b]12;#00ff00\x07",)
    screen.feed(b"\x1b[2J\x1b[Hfresh")
    assert screen.lines()[0] == "fresh"
    screen.feed(b"\x1b[?1049halt")
    screen.resize(4, 10)
    screen.feed(b"\x1b[?1049l")
    assert screen.lines()[0] == "fresh"


def test_session_keeps_shell_state() -> None:
    with TerminalSession(("sh", "-i"), rows=6, columns=40) as terminal:
        terminal.send_text("printf 'first-turn\\n'")
        terminal.key("ENTER")
        terminal.wait_for("first-turn")
        terminal.send_text("printf 'second-turn\\n'")
        terminal.key("ENTER")
        terminal.wait_for("second-turn")
        assert terminal.alive
        terminal.resize(8, 50)
        assert terminal.screen.rows == 8
        assert terminal.screen.columns == 50
        terminal.key("CTRL-D")
        terminal.read(0.2)
        terminal.close()


def test_controller_output_is_opt_in() -> None:
    script = Path(__file__).with_name("terminal-session.py")
    process = subprocess.Popen(
        [sys.executable, str(script), "--rows", "6", "--columns", "40", "--", "sh", "-i"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        text=True,
    )
    assert process.stdin is not None
    assert process.stdout is not None
    try:
        process.stdin.write(json.dumps({"op": "send", "data": "printf 'controller\\n'"}) + "\n")
        process.stdin.flush()
        first = json.loads(process.stdout.readline())
        assert "output_base64" not in first
        process.stdin.write(json.dumps({"op": "key", "key": "ENTER", "include_output": True}) + "\n")
        process.stdin.flush()
        second = json.loads(process.stdout.readline())
        assert isinstance(second.get("output_base64"), str)
        process.stdin.write(json.dumps({"op": "close"}) + "\n")
        process.stdin.flush()
        assert json.loads(process.stdout.readline())["ok"] is True
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=5)


def main() -> int:
    test_screen_sequences()
    test_session_keeps_shell_state()
    test_controller_output_is_opt_in()
    print("terminal session: ok (persistent PTY, ANSI screen, key input, resize)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
