#!/usr/bin/env python3
"""Exercise explicit history search in the persistent Zsh editor."""
from __future__ import annotations

import subprocess
import tempfile
from pathlib import Path

from terminal_config import terminal_config
from terminal_session import TerminalSession

ROOT = Path(__file__).resolve().parent.parent


def open_shell(zdotdir: str, config_root: str) -> TerminalSession:
    return TerminalSession(
        ("zsh", "-di"), rows=24, columns=80, cwd=str(ROOT),
        env={"ZDOTDIR": zdotdir, "XDG_CONFIG_HOME": config_root,
             "SOPRA_BIN": str(ROOT / "target/debug/sopra-worker")},
    )


def line(terminal: TerminalSession) -> str:
    return terminal.screen.lines()[terminal.screen.row].rstrip()


def search_and_submit(zdotdir: str, config_root: str) -> None:
    with open_shell(zdotdir, config_root) as terminal:
        terminal.wait_for("test $ ", timeout=20)
        terminal.send_text("history-")
        terminal.send(b"\x12")
        terminal.read(0.4)
        assert "History: 1/2" in terminal.screen.text(), terminal.screen.text()
        assert "Enter select" in terminal.screen.text(), terminal.screen.text()
        terminal.key("DOWN")
        terminal.key("ENTER")
        terminal.read(0.3)
        assert line(terminal) == "test $ echo history-older", terminal.screen.text()
        assert "history-older" not in terminal.screen.lines(), terminal.screen.text()
        assert "History:" not in terminal.screen.text(), terminal.screen.text()
        terminal.key("ENTER")
        terminal.read(0.5)
        assert "history-older" in terminal.screen.lines(), terminal.screen.text()


def cancellation_and_no_matches(zdotdir: str, config_root: str) -> None:
    with open_shell(zdotdir, config_root) as terminal:
        terminal.wait_for("test $ ", timeout=20)
        terminal.send_text("unfinished draft")
        terminal.key("LEFT")
        terminal.read(0.2)
        column = terminal.screen.column
        terminal.send(b"\x12")
        terminal.read(0.2)
        assert "No matches" in terminal.screen.text(), terminal.screen.text()
        terminal.key("ENTER")
        terminal.read(0.2)
        assert "No matches" in terminal.screen.text(), terminal.screen.text()
        terminal.key("CTRL-U")
        terminal.paste("世界")
        terminal.read(0.2)
        assert "echo café 世界" in terminal.screen.text(), terminal.screen.text()
        terminal.resize(24, 40)
        terminal.read(0.2)
        terminal.resize(24, 80)
        terminal.read(0.2)
        terminal.key("ESC")
        terminal.read(0.2)
        assert line(terminal) == "test $ unfinished draft", terminal.screen.text()
        assert terminal.screen.column == column, terminal.screen.text()
        terminal.send(b"\x12")
        terminal.key("CTRL-C")
        terminal.read(0.2)
        assert line(terminal) == "test $ unfinished draft", terminal.screen.text()
        assert terminal.screen.column == column, terminal.screen.text()


def independent_of_completions(zdotdir: str, config_root: str) -> None:
    with open_shell(zdotdir, config_root) as terminal:
        terminal.wait_for("test $ ", timeout=20)
        terminal.send(b"\x12")
        terminal.read(0.5)
        assert "History: 1/3" in terminal.screen.text(), terminal.screen.text()
        terminal.key("CTRL-U")
        terminal.send_text("CAFÉ")
        terminal.read(0.2)
        assert "History: 1/1" in terminal.screen.text(), terminal.screen.text()
        terminal.key("TAB")
        terminal.read(0.2)
        assert line(terminal) == "test $ echo café 世界", terminal.screen.text()


def pending_completion_stays_out_of_search(zdotdir: str, config_root: str) -> None:
    with open_shell(zdotdir, config_root) as terminal:
        terminal.wait_for("test $ ", timeout=20)
        terminal.send_text("echo")
        terminal.send(b"\x12")
        terminal.read(1.0)
        assert "History: 1/3" in terminal.screen.text(), terminal.screen.text()
        terminal.send(b"\x12")
        terminal.read(0.2)
        assert "History: 2/3" in terminal.screen.text(), terminal.screen.text()
        terminal.key("ESC")
        terminal.read(0.2)
        assert "History:" not in terminal.screen.text(), terminal.screen.text()


def main() -> None:
    subprocess.run(["cargo", "build", "--quiet", "--locked", "--bin", "sopra-worker"],
                   cwd=ROOT, check=True)
    with tempfile.TemporaryDirectory(prefix="sopra-history-search-") as zdotdir:
        (Path(zdotdir) / ".zshrc").write_text(
            f"HISTFILE='{zdotdir}/history'\nHISTSIZE=200\nSAVEHIST=0\n"
            f"source '{ROOT}/zsh/editor.zsh'\n"
            "print -s -- 'echo history-older'\nprint -s -- 'echo history-newest'\n"
            "print -s -- 'echo café 世界'\n")
        config = "[prompt]\nleft = 'test $ '\nright = ''\n[editor]\ncursor_animation = false\n"
        with terminal_config(config) as config_root:
            pending_completion_stays_out_of_search(zdotdir, config_root)
            search_and_submit(zdotdir, config_root)
            cancellation_and_no_matches(zdotdir, config_root)
            independent_of_completions(zdotdir, config_root)
        config += "autosuggestions = false\n[completion]\nenabled = false\n"
        with terminal_config(config) as config_root:
            independent_of_completions(zdotdir, config_root)
    print("history search: ok (filter, selection, cancellation, resize, disabled completions)")


if __name__ == "__main__":
    main()
