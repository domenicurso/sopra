"""Isolated hosted shells for PTY boundary and scrollback regression tests."""
from __future__ import annotations

from contextlib import contextmanager
from pathlib import Path
from typing import Iterator
from collections.abc import Callable
import tempfile
import os
import shlex
import time

from terminal_session import TerminalSession

ROOT = Path(__file__).resolve().parent.parent
CONFIG = '''[prompt]
left = "host\\n$ "
right = "rprompt"
[editor]
cursor_animation = false
autosuggestions = false
[completion]
enabled = false
'''


@contextmanager
def hosted() -> Iterator[tuple[TerminalSession, Path]]:
    with tempfile.TemporaryDirectory(prefix="sopra-host-test-") as directory:
        root = Path(directory)
        config = root / "sopra"
        config.mkdir()
        (config / "config.toml").write_text(CONFIG)
        (root / ".zshrc").write_text("HISTFILE=/dev/null\nSAVEHIST=0\n")
        bin_dir = root / "bin"
        bin_dir.mkdir()
        for name in ["open", "xdg-open"]:
            opener = bin_dir / name
            opener.write_text(f"#!/bin/sh\nprintf '%s' \"$1\" > {shlex.quote(str(root / 'opened'))}\n")
            opener.chmod(0o755)
        with TerminalSession(
            (str(ROOT / "target/debug/sopra-worker"), "shell"),
            cwd=str(root), rows=24, columns=120,
            env={"ZDOTDIR": directory, "XDG_CONFIG_HOME": directory, "PATH": f"{bin_dir}:{os.environ['PATH']}"},
        ) as terminal:
            terminal.wait_for("rprompt", timeout=5)
            terminal.read(0.1)
            yield terminal, root


def submit(terminal: TerminalSession, command: str) -> None:
    terminal.send_text(command)
    terminal.read(0.1)
    terminal.key("ENTER")
    terminal.read(0.4)


def browse(terminal: TerminalSession) -> None:
    # The pointer may be over the live prompt; the wheel must still scroll history.
    terminal.send(b"\x1b[<64;10;24M")
    terminal.wait_for("Session history")
    terminal.read(0.1)


def wait_until(terminal: TerminalSession, predicate: Callable[[], bool], timeout: float = 3) -> None:
    # Callers assert process/file boundaries that aren't represented by screen text.
    deadline = time.monotonic() + timeout
    while not predicate():
        if time.monotonic() >= deadline:
            raise AssertionError(terminal.screen.text())
        terminal.read(0.05)
