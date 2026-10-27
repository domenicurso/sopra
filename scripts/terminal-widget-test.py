#!/usr/bin/env python3
"""Exercise live and interactive prompt widgets through the Zsh bridge."""
from __future__ import annotations

import re
import subprocess
import tempfile
from pathlib import Path

from terminal_session import TerminalSession


def exercise_widgets(root: Path, directory: Path) -> None:
    environment = {
        "SOPRA_REPO_ROOT": str(root),
        "SOPRA_BIN": str(root / "target/debug/sopra-worker"),
        "ZDOTDIR": str(root / "examples/isolated"),
        "SOPRA_PROMPT": "{path} $ ",
        "SOPRA_RPROMPT": "{git} {clock}",
        "SOPRA_TRANSIENT_PROMPT": "done> ",
        "SOPRA_COMPLETION_ENABLED": "0",
        "SOPRA_CURSOR_ANIMATION": "0",
        "SOPRA_SYNTAX_HIGHLIGHTING": "0",
    }
    with TerminalSession(("zsh", "-di"), rows=20, columns=100, cwd=str(directory), env=environment) as terminal:
        terminal.wait_for(" $ ", timeout=20)
        terminal.read(0.5)
        assert "feature%clock" in terminal.screen.text(), terminal.screen.lines()
        first = re.search(r"\b\d\d:\d\d:\d\d\b", terminal.screen.text())
        assert first, terminal.screen.lines()
        terminal.read(1.5)
        second = re.search(r"\b\d\d:\d\d:\d\d\b", terminal.screen.text())
        assert second and first.group() != second.group(), terminal.screen.lines()
        terminal.resize(20, 80)
        terminal.read(0.3)
        terminal.resize(20, 100)
        terminal.read(0.3)
        terminal.send_text("echo preserved")
        terminal.read(0.2)
        terminal.send(b"\x07\t")
        terminal.read(0.2)
        assert f"[{directory.parent}]" in terminal.screen.text(), terminal.screen.lines()
        terminal.key("ENTER")
        terminal.read(1)
        text = terminal.screen.text()
        assert str(directory.parent) in text and "echo preserved" in text, text
        terminal.key("ENTER")
        terminal.read(0.5)
        assert "done> echo preserved" in terminal.screen.text()
        terminal.send_text("pwd")
        terminal.key("ENTER")
        terminal.read(0.5)
        assert str(directory.parent) in terminal.screen.lines(), terminal.screen.lines()
        terminal.send(b"\x07\t\r")
        terminal.read(0.8)
        terminal.send_text("pwd")
        terminal.key("ENTER")
        terminal.read(0.5)
        assert str(directory.parent.parent) in terminal.screen.lines(), terminal.screen.lines()


def main() -> None:
    root = Path(__file__).resolve().parent.parent
    subprocess.run(["cargo", "build", "--quiet", "--locked", "--bin", "sopra-worker"], cwd=root, check=True)
    with tempfile.TemporaryDirectory(prefix="sw-", dir="/tmp") as temporary:
        directory = Path(temporary).resolve() / "parent space" / "child"
        directory.mkdir(parents=True)
        subprocess.run(["git", "init", "--quiet", str(directory)], check=True)
        subprocess.run(["git", "-C", str(directory), "symbolic-ref", "HEAD", "refs/heads/feature%clock"], check=True)
        exercise_widgets(root, directory)
    print("prompt widgets: ok (live clock, parent selection, preserved input)")


if __name__ == "__main__":
    main()
