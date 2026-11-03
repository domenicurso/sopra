#!/usr/bin/env python3
"""Exercise explicit newlines and styled multiline transcripts through Zsh."""
from __future__ import annotations

import subprocess
import tempfile
from pathlib import Path

from terminal_config import terminal_config
from terminal_session import TerminalSession

ROOT = Path(__file__).resolve().parent.parent
CONFIG = """[prompt]
left = 'PRIMARY $ '
right = ''
transient = '$ '
secondary = '[dim]CONT >[/dim] '
[completion]
enabled = false
[editor]
cursor_animation = false
"""


def exercise(config_root: str, directory: str, width: int) -> None:
    env = {
        "SOPRA_REPO_ROOT": str(ROOT), "SOPRA_BIN": str(ROOT / "target/debug/sopra-worker"),
        "ZDOTDIR": str(ROOT / "examples/isolated"), "XDG_CONFIG_HOME": config_root,
    }
    with TerminalSession(("zsh", "-di"), rows=24, columns=width, cwd=directory, env=env) as terminal:
        terminal.wait_for("PRIMARY", timeout=20)
        terminal.send_text("setopt promptsubst")
        terminal.key("ENTER")
        terminal.read(0.3)
        terminal.send_text("echo first > first")
        terminal.key("SHIFT-ENTER")
        terminal.read(0.2)
        assert not (Path(directory) / "first").exists(), terminal.screen.text()
        assert "CONT >" in terminal.screen.text(), terminal.screen.text()
        terminal.send_text("echo second")
        terminal.key("ENTER")
        terminal.read(0.4)
        lines = terminal.screen.lines()
        assert "CONT > echo second" in lines and "second" in lines, lines
        assert (Path(directory) / "first").read_text() == "first\n"
        assert b"\x1b[>1u" in terminal.output and b"\x1b[<u" in terminal.output
        # The contiguous styled prefix is the shell transcript, not a painted frame.
        assert b"\x1b[2mCONT >\x1b[0m" in terminal.output
        terminal.send_text("printf '%s\\n' '$(touch unexpected)%F{red}'")
        terminal.key("SHIFT-ENTER")
        terminal.send_text("echo safe")
        terminal.key("ENTER")
        terminal.read(0.4)
        assert not (Path(directory) / "unexpected").exists()
        assert "safe" in terminal.screen.lines(), terminal.screen.text()
        terminal.send_text("echo cancelled")
        terminal.send(b"\x1b[99;5u")  # Enhanced Ctrl-C must still cancel.
        terminal.read(0.3)
        assert "cancelled" not in terminal.screen.lines(), terminal.screen.text()
        terminal.send_text("echo recovery")
        terminal.key("ENTER")
        terminal.read(0.3)
        assert "recovery" in terminal.screen.lines(), terminal.screen.text()


def main() -> None:
    subprocess.run(["cargo", "build", "--quiet", "--locked", "--bin", "sopra-worker"], cwd=ROOT, check=True)
    with terminal_config(CONFIG) as config_root, tempfile.TemporaryDirectory(prefix="sopra-newline-") as directory:
        for width in (80, 25):
            (Path(directory) / "first").unlink(missing_ok=True)
            exercise(config_root, directory, width)
    with terminal_config(CONFIG.replace("transient = '$ '", 'transient = "T\\n$ "')) as config_root, tempfile.TemporaryDirectory(prefix="sopra-newline-") as directory:
        exercise(config_root, directory, 80)
    print("explicit newlines and styled secondary transcripts: ok")


if __name__ == "__main__":
    main()
