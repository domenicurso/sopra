#!/usr/bin/env python3
"""Exercise live Zsh history suggestions without modifying the user's history."""
from __future__ import annotations

import re
import subprocess
import tempfile
from pathlib import Path

from terminal_config import terminal_config
from terminal_session import TerminalSession

ROOT = Path(__file__).resolve().parent.parent


def environment(zdotdir: str, config_root: str) -> dict[str, str]:
    return {"ZDOTDIR": zdotdir, "XDG_CONFIG_HOME": config_root,
            "SOPRA_BIN": str(ROOT / "target/debug/sopra-worker")}


def current_line(terminal: TerminalSession) -> str:
    lines = terminal.screen.lines()
    return re.sub(r"  \d+s ago$", "  recent", lines[terminal.screen.row].rstrip())


def exercise(zdotdir: str, config_root: str) -> None:
    with TerminalSession(("zsh", "-di"), rows=24, columns=80, cwd=str(ROOT),
                         env=environment(zdotdir, config_root)) as terminal:
        terminal.wait_for("test $ ", timeout=20)
        terminal.send_text("echo history-")
        terminal.read(0.3)
        assert current_line(terminal) == "test $ echo history-newest  recent", terminal.screen.text()
        assert terminal.screen.column == len("test $ echo history-")
        terminal.resize(24, 25)
        terminal.read(0.2)
        terminal.resize(24, 80)
        terminal.read(0.2)
        assert current_line(terminal) == "test $ echo history-newest  recent", terminal.screen.text()
        terminal.key("RIGHT")
        terminal.key("ENTER")
        terminal.read(0.4)
        assert "history-newest" in terminal.screen.lines(), terminal.screen.text()
        terminal.send_text("echo history-")
        terminal.key("TAB")
        terminal.read(0.2)
        assert current_line(terminal) == "test $ echo history-newest", terminal.screen.text()
        assert terminal.screen.column == len("test $ echo history-newest")
        terminal.key("CTRL-U")
        terminal.send_text("echo history-")
        terminal.key("ESC")
        terminal.read(0.2)
        assert current_line(terminal) == "test $ echo history-", terminal.screen.text()
        terminal.key("ENTER")
        terminal.read(0.4)
        assert "history-" in terminal.screen.lines(), terminal.screen.text()
        terminal.send_text("echo history-o")
        terminal.key("END")
        terminal.read(0.2)
        assert terminal.screen.column == len("test $ echo history-older")
        terminal.key("CTRL-U")
        terminal.send_text("echo session-only")
        terminal.key("ENTER")
        terminal.read(0.3)
        terminal.send_text("echo session-")
        terminal.read(0.2)
        assert current_line(terminal) == "test $ echo session-only  recent", terminal.screen.text()
        terminal.key("CTRL-U")
        terminal.send_text("echo not-a-match")
        terminal.read(0.2)
        assert current_line(terminal) == "test $ echo not-a-match", terminal.screen.text()


def cycle_and_recall(zdotdir: str, config_root: str) -> None:
    with TerminalSession(("zsh", "-di"), rows=24, columns=80, cwd=str(ROOT),
                         env=environment(zdotdir, config_root)) as terminal:
        terminal.wait_for("test $ ", timeout=20)
        terminal.send_text("echo history-")
        terminal.read(0.2)
        assert re.search(r"\d+s ago", terminal.screen.text()), terminal.screen.text()
        terminal.key("DOWN")
        terminal.read(0.2)
        assert current_line(terminal) == "test $ echo history-older  recent", terminal.screen.text()
        assert terminal.screen.column == len("test $ echo history-")
        terminal.key("CTRL-L")
        terminal.key("UP")
        terminal.read(0.2)
        assert current_line(terminal) == "test $ echo history-newest  recent", terminal.screen.text()
        assert "Tab to accept" not in terminal.screen.text(), terminal.screen.text()
        terminal.key("DOWN")
        terminal.read(0.2)
        assert current_line(terminal) == "test $ echo history-older  recent", terminal.screen.text()
        terminal.send(b"\x1b[1;3A")
        terminal.read(0.2)
        assert current_line(terminal) == "test $ echo history-newest  recent", terminal.screen.text()
        terminal.key("CTRL-U")
        terminal.send_text("echo unseen-draft")
        terminal.key("CTRL-L")
        terminal.read(0.2)
        row = terminal.screen.row
        terminal.key("UP")
        terminal.read(0.2)
        assert current_line(terminal) == "test $ echo history-newest", terminal.screen.text()
        assert terminal.screen.row == row, terminal.screen.text()
        terminal.key("DOWN")
        terminal.read(0.2)
        assert current_line(terminal) == "test $ echo unseen-draft", terminal.screen.text()
        assert terminal.screen.row == row, terminal.screen.text()
        assert terminal.screen.text().count("test $") == 1, terminal.screen.text()


def disabled(zdotdir: str, config_root: str) -> None:
    with TerminalSession(("zsh", "-di"), rows=24, columns=80, cwd=str(ROOT),
                         env=environment(zdotdir, config_root)) as terminal:
        terminal.wait_for("test $ ", timeout=20)
        terminal.send_text("echo history-")
        terminal.read(0.2)
        assert current_line(terminal) == "test $ echo history-", terminal.screen.text()
        terminal.key("RIGHT")
        assert terminal.screen.column == len("test $ echo history-")
        assert re.search(r"\d+s ago", terminal.screen.text()), terminal.screen.text()


def main() -> None:
    subprocess.run(["cargo", "build", "--quiet", "--locked", "--bin", "sopra-worker"], cwd=ROOT, check=True)
    with tempfile.TemporaryDirectory(prefix="sopra-history-shell-") as zdotdir:
        (Path(zdotdir) / ".zshrc").write_text(
            f"HISTFILE='{zdotdir}/history'\nHISTSIZE=200\nSAVEHIST=0\nsource '{ROOT}/zsh/editor.zsh'\n"
            "print -s -- 'echo history-older'\nprint -s -- 'echo history-newest'\n")
        config = "[prompt]\nleft = 'test $ '\nright = ''\n[editor]\ncursor_animation = false\n"
        with terminal_config(config) as config_root:
            exercise(zdotdir, config_root)
            cycle_and_recall(zdotdir, config_root)
        with terminal_config(config + "autosuggestions = false\n") as config_root:
            disabled(zdotdir, config_root)
    print("history: ok (live snapshot, inline acceptance, menu, resize, dismissal, configuration)")


if __name__ == "__main__":
    main()
