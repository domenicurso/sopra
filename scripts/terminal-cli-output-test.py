#!/usr/bin/env python3
"""Check width-aware CLI output and styling through a real terminal descriptor."""
from __future__ import annotations
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time
from terminal_session import TerminalSession

ROOT = Path(__file__).resolve().parent.parent
BINARY = ROOT / "target/debug/sopra-worker"
SGR = re.compile(r"\x1b\[[0-9;]*m")


def capture(arguments: list[str], width: int, env: dict[str, str]) -> str:
    with TerminalSession((str(BINARY), *arguments), rows=80, columns=width, env=env) as terminal:
        deadline = time.monotonic() + 5
        while terminal.alive and time.monotonic() < deadline:
            terminal.read(0.05)
        terminal.read(0.1)
        assert not terminal.alive, arguments
        return terminal.output.decode().replace("\r", "")


def exercise(directory: Path) -> None:
    env = {"TERM": "xterm-256color", "NO_COLOR": "", "_SOPRA_SERVER_SOCKET": "",
           "XDG_CONFIG_HOME": str(directory)}
    for width in [24, 40, 64, 80, 120]:
        help_text = capture(["--help"], width, env)
        assert "\x1b[1;36mrefresh" in help_text and "\x1b[2m" in help_text
        plain = SGR.sub("", help_text)
        assert all(len(line) < width for line in plain.splitlines()), (width, plain)
        assert "config edit" in plain and "widgets list" in plain
        table = capture(["widgets", "list"], width, env)
        plain = SGR.sub("", table)
        assert "clock" in plain and "built-in" in plain
        assert "\t" not in table
        assert all(len(line) < width for line in plain.splitlines()), (width, plain)
    config = capture(["config", "show"], 100, env)
    assert "\x1b[1;35m[editor]" in config
    assert "\x1b[36mcursor_animation" in config
    assert "\x1b[35mtrue" in config
    for args in [["--help"], ["widgets", "list"], ["config", "show"]]:
        assert "\x1b" not in capture(args, 80, {**env, "NO_COLOR": "1"})
        assert "\x1b" not in capture(args, 80, {**env, "TERM": "dumb"})
        result = subprocess.run([str(BINARY), *args], env={**os.environ, **env}, capture_output=True, text=True, check=True)
        assert "\x1b" not in result.stdout


def main() -> None:
    subprocess.run(["cargo", "build", "--quiet", "--locked"], cwd=ROOT, check=True)
    with tempfile.TemporaryDirectory(prefix="sopra-cli-output-") as temporary:
        exercise(Path(temporary))
    print("CLI output: ok (terminal widths, aligned tables, TOML highlighting, plain pipes, NO_COLOR)")


if __name__ == "__main__":
    main()
