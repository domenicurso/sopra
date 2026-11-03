#!/usr/bin/env python3
"""Exercise discovery, nested acceptance, and cache freshness through an isolated Zsh."""
from __future__ import annotations
import os
from pathlib import Path
import subprocess
import tempfile
import time
from terminal_session import TerminalSession

ROOT = Path(__file__).resolve().parent.parent


def wait_screen(terminal: TerminalSession, expected: str, timeout: float = 12) -> str:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        terminal.read(0.1)
        screen = terminal.screen.text()
        if expected in screen:
            return screen
    raise AssertionError(f"Missing {expected!r}:\n{terminal.screen.text()}")


def show(terminal: TerminalSession, line: str, expected: str) -> str:
    terminal.key("CTRL-C")
    terminal.paste(line)
    return wait_screen(terminal, expected)


def fixture(path: Path, action: str) -> None:
    path.write_text(f'''#!/bin/sh
case "$1" in
  --help|-h)
    printf 'Usage: demo-cli <command>\\n\\nCommands:\\n  group      Manage fixtures\\n  widgets list  List fixtures\\n  {action}   Current build action\\n  here       Directory %s\\n' "${{PWD##*/}}" ;;
  group)
    printf 'Usage: demo-cli group <command>\\n\\nCommands:\\n  build  Build fixture\\n  test   Test fixture\\n\\nOptions:\\n  --mode <fast|slow>  Choose mode\\n  --tag <tag>  Choose a live value\\n' ;;
  *) printf 'demo-cli: unknown command\\n' >&2; exit 1 ;;
esac
''')
    path.chmod(0o755)


def own_commands(terminal: TerminalSession) -> None:
    screen = show(terminal, "sopra dev ", "Build the checkout")
    for name in ("build", "check", "test", "shell", "link", "unlink", "path"):
        assert name in screen, screen
    assert "unknown command" not in screen, screen
    show(terminal, "sopra debug ", "Show byte ranges")
    flags = show(terminal, "sopra debug completions --", "--chunk")
    assert "--root" in flags and "--cwd" in flags, flags
    show(terminal, "sopra wid", "widgets list")
    terminal.key("TAB")
    wait_screen(terminal, "$ sopra widgets list")


def repeated_tabs(terminal: TerminalSession) -> None:
    for line in ("", "   ", "sopra-no-such-command-94815"):
        terminal.key("CTRL-C")
        terminal.paste(line)
        terminal.read(0.5)
        before = terminal.screen.text(trim=False)
        cursor = (terminal.screen.row, terminal.screen.column)
        output_start = len(terminal.output)
        for _ in range(20):
            terminal.key("TAB")
            terminal.read(0.03)
        terminal.read(0.3)
        assert terminal.screen.text(trim=False) == before, terminal.screen.text()
        assert (terminal.screen.row, terminal.screen.column) == cursor, terminal.screen.text()
        # Restoring the native cursor means Tab exited the editor and handed off to ZLE.
        assert b"\x1b[?25h" not in terminal.output[output_start:]


def nested_commands(terminal: TerminalSession) -> None:
    show(terminal, "demo-cli gro", "Manage fixtures")
    terminal.key("TAB")
    wait_screen(terminal, "$ demo-cli group")
    terminal.send_text(" ")
    wait_screen(terminal, "Build fixture")
    screen = show(terminal, "demo-cli group --mode ", "slow")
    assert "fast" in screen, screen
    show(terminal, "demo-cli group --tag sp", "space value")
    terminal.key("TAB")
    wait_screen(terminal, "$ demo-cli group --tag space\\ value")
    show(terminal, "demo-cli wid", "widgets list")
    terminal.key("TAB")
    wait_screen(terminal, "$ demo-cli widgets list")


def cache_freshness(terminal: TerminalSession, executable: Path, directory: Path) -> None:
    show(terminal, "demo-cli ", "Current build action")
    fixture(executable, "replacement-action")
    graph = subprocess.check_output([str(ROOT / "target/debug/sopra-worker"), "debug", "completions", "--root", str(executable)], text=True)
    assert "replacement-action" in graph, graph
    screen = show(terminal, "demo-cli repl", "replacement-action")
    assert "unknown command" not in screen, screen
    directory = directory / "new-directory"
    directory.mkdir()
    terminal.key("CTRL-C")
    terminal.paste(f"cd '{directory}'")
    terminal.key("ENTER")
    terminal.read(0.5)
    screen = show(terminal, "demo-cli her", f"Directory {directory.name}")
    assert "here" in screen, screen


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="sopra-completion-terminal-") as temporary:
        directory = Path(temporary)
        tools = directory / "tools"
        tools.mkdir()
        executable = tools / "demo-cli"
        fixture(executable, "original-action")
        config = directory / "config/sopra"
        config.mkdir(parents=True)
        (directory / "values.json").write_text('{"values":{"space value":"fixture","second":"fixture"}}')
        (config / "completions.toml").write_text('[[provider]]\nid="terminal-live"\ncommand="demo-cli"\npath=["group"]\noption="--tag"\njson_file="values.json"\npointer="/values"\nroot_marker="values.json"\nwatch=["values.json"]\n')
        (config / "config.toml").write_text('[prompt]\nleft="$ "\nright=""\n')
        shell = directory / "shell"
        shell.mkdir()
        (shell / ".zshrc").write_text(f"HISTFILE=/dev/null\nSAVEHIST=0\nsource '{ROOT}/zsh/editor.zsh'\n")
        env = dict(os.environ, XDG_CONFIG_HOME=str(config.parent),
                   ZDOTDIR=str(shell), SOPRA_REPO_ROOT=str(ROOT),
                   SOPRA_BIN=str(ROOT / "target/debug/sopra-worker"),
                   PATH=str(tools) + os.pathsep + os.environ.get("PATH", ""))
        with TerminalSession(("zsh", "-di"), cwd=str(directory),
                             rows=32, columns=140, env=env) as terminal:
            terminal.wait_for("$ ", timeout=30)
            repeated_tabs(terminal)
            own_commands(terminal)
            nested_commands(terminal)
            cache_freshness(terminal, executable, directory)
    print("Completion terminal checks passed")


if __name__ == "__main__":
    main()
