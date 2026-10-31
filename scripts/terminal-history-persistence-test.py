#!/usr/bin/env python3
"""Verify history survives shells and concurrent sessions without user files."""
from __future__ import annotations

import subprocess
import tempfile
from pathlib import Path

from terminal_config import terminal_config
from terminal_session import TerminalSession

ROOT = Path(__file__).resolve().parent.parent


def shell(directory: Path, config_root: str) -> TerminalSession:
    return TerminalSession(
        ("zsh", "-di"), rows=24, columns=80, cwd=str(ROOT),
        env={"ZDOTDIR": str(directory), "XDG_CONFIG_HOME": config_root,
             "SOPRA_BIN": str(ROOT / "target/debug/sopra-worker")})


def submit(terminal: TerminalSession, command: str) -> None:
    terminal.send_text(command)
    terminal.key("ENTER")
    terminal.read(0.5)


def assert_suggestion(terminal: TerminalSession, prefix: str, command: str) -> None:
    terminal.send_text(prefix)
    terminal.read(0.3)
    assert command in terminal.screen.lines()[terminal.screen.row], terminal.screen.text()
    assert terminal.screen.column == len("persist $ " + prefix), terminal.screen.text()
    terminal.key("CTRL-U")


def across_sessions(directory: Path, config_root: str) -> None:
    history = directory / ".zsh_history"
    with shell(directory, config_root) as first:
        first.wait_for("persist $ ", timeout=20)
        submit(first, "echo persisted-first")
        assert "echo persisted-first" in history.read_text(), history.read_text()
        with shell(directory, config_root) as second:
            second.wait_for("persist $ ", timeout=20)
            assert_suggestion(second, "echo persisted-f", "echo persisted-first")
            submit(second, "echo persisted-second")
            submit(second, "exit")
        submit(first, "exit")
    with shell(directory, config_root) as third:
        third.wait_for("persist $ ", timeout=20)
        assert_suggestion(third, "echo persisted-f", "echo persisted-first")
        assert_suggestion(third, "echo persisted-s", "echo persisted-second")
        submit(third, "exit")
    contents = history.read_text()
    assert "echo persisted-first" in contents and "echo persisted-second" in contents, contents
    assert contents.startswith(": "), contents


def respects_configuration(directory: Path) -> None:
    history = directory / "disabled-history"
    result = subprocess.run(
        ["zsh", "-fc", 'HISTFILE=$1; HISTSIZE=200; SAVEHIST=0; '
         'setopt SHARE_HISTORY; source "$2/zsh/history.zsh"; '
         '[[ $HISTFILE == $1 && $HISTSIZE == 200 && $SAVEHIST == 0 '
         '&& -o SHARE_HISTORY && ! -o INC_APPEND_HISTORY_TIME ]]',
         "history-test", str(history), str(ROOT)], check=False)
    assert result.returncode == 0, "Existing history configuration was changed"
    assert not history.exists()


def main() -> None:
    config = "[prompt]\nleft = 'persist $ '\nright = ''\n[completion]\nenabled = false\n[editor]\ncursor_animation = false\n"
    with tempfile.TemporaryDirectory(prefix="sopra-history-persistence-") as temporary:
        directory = Path(temporary)
        (directory / ".zshrc").write_text(f"source '{ROOT}/zsh/editor.zsh'\n")
        with terminal_config(config) as config_root:
            across_sessions(directory, config_root)
        respects_configuration(directory)
    print("history persistence: ok (restart, concurrent shells, timestamps, explicit opt-out)")


if __name__ == "__main__":
    main()
