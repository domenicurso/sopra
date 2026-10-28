#!/usr/bin/env python3
"""Exercise live and interactive prompt widgets through the Zsh bridge."""
from __future__ import annotations

import re
import subprocess
import tempfile
from pathlib import Path

from terminal_session import TerminalSession


def exercise_widgets(root: Path, directory: Path) -> None:
    config = directory.parent / "widget-config" / "sopra"
    config.mkdir(parents=True)
    (config / "config.toml").write_text(
        "[prompt]\nleft = '{path} $ '\nright = '[if git]{git}[dim] : [/][/]{clock}'\n"
        "transient = '[dim]done>[/] '\n[widgets.clock]\nformat = '%H[dim]:[/]%M[dim]:[/]%S'\n"
        "[editor]\ncompletion = false\ncursor_animation = false\nsyntax_highlighting = false\n"
    )
    environment = {
        "SOPRA_REPO_ROOT": str(root),
        "SOPRA_BIN": str(root / "target/debug/sopra-worker"),
        "ZDOTDIR": str(root / "examples/isolated"),
        "XDG_CONFIG_HOME": str(config.parent),
        "SOPRA_CLOCK_FORMAT": "WRONG",
        "SOPRA_GIT_STATUS_FORMAT": "WRONG",
    }
    with TerminalSession(("zsh", "-di"), rows=20, columns=100, cwd=str(directory), env=environment) as terminal:
        terminal.wait_for(" $ ", timeout=20)
        terminal.read(0.5)
        assert "feature%clock : " in terminal.screen.text(), terminal.screen.lines()
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
        terminal.send(b"\x07\t\x1b[Z\t")
        terminal.read(0.2)
        assert str(directory) in terminal.screen.text(), terminal.screen.lines()
        terminal.key("ENTER")
        terminal.read(1)
        text = terminal.screen.text()
        assert str(directory.parent) in text and "echo preserved" in text, text
        assert "[dim]" not in text and " : " not in text, text
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


def exercise_multiline_directory(root: Path, directory: Path) -> None:
    config = directory.parent / "path-config" / "sopra"
    config.mkdir(parents=True)
    (config / "config.toml").write_text(
        "[prompt]\nleft = \"{path}\\n$ \"\nright = ''\ntransient = '$ '\n"
        "[editor]\ncompletion = false\ncursor_animation = false\n"
        "[widgets.path]\nbefore = '[green]{text}[/]'\n"
        "selected = '[yellow][reverse]{text}[/][/]'\nafter = '[red][dim]{text}[/]'\n"
    )
    environment = {
        "XDG_CONFIG_HOME": str(config.parent),
        "SOPRA_REPO_ROOT": str(root), "SOPRA_BIN": str(root / "target/debug/sopra-worker"),
        "ZDOTDIR": str(root / "examples/isolated"),
        "SOPRA_PATH_BEFORE": "WRONG", "SOPRA_PATH_SELECTED": "WRONG", "SOPRA_PATH_AFTER": "WRONG",
    }
    command = ("zsh", "-di")
    with TerminalSession(command, rows=20, columns=100, cwd=str(directory), env=environment) as terminal:
        terminal.wait_for(str(directory), timeout=20)
        terminal.send_text("echo hello")
        terminal.key("ENTER")
        terminal.wait_for("hello", timeout=10)
        terminal.read(0.4)
        terminal.send_text("echo preserved")
        terminal.read(0.1)
        terminal.send(b"\x07\t")
        terminal.read(0.2)
        output = bytes(terminal.output)
        assert b"\x1b[32m" in output and b"\x1b[33m" in output and b"\x1b[31m" in output, output
        terminal.key("ENTER")
        terminal.read(0.8)
        lines = terminal.screen.lines()
        output_row = lines.index("hello")
        assert lines[output_row + 1] == str(directory.parent), lines
        assert lines[output_row + 2].startswith("$ echo preserved"), lines
        terminal.key("ENTER")
        terminal.read(0.4)
        assert "$ echo preserved" in terminal.screen.lines(), terminal.screen.lines()


def main() -> None:
    root = Path(__file__).resolve().parent.parent
    subprocess.run(["cargo", "build", "--quiet", "--locked", "--bin", "sopra-worker"], cwd=root, check=True)
    with tempfile.TemporaryDirectory(prefix="sw-", dir="/tmp") as temporary:
        directory = Path(temporary).resolve() / "parent space" / "child"
        directory.mkdir(parents=True)
        subprocess.run(["git", "init", "--quiet", str(directory)], check=True)
        subprocess.run(["git", "-C", str(directory), "symbolic-ref", "HEAD", "refs/heads/feature%clock"], check=True)
        exercise_widgets(root, directory)
        exercise_multiline_directory(root, directory)
    print("prompt widgets: ok (live clock, parent selection, preserved input)")


if __name__ == "__main__":
    main()
