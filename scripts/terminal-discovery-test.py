#!/usr/bin/env python3
"""Verify slow background help cannot block a new foreground command or live values."""
from __future__ import annotations
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
from terminal_session import TerminalSession

ROOT = Path(__file__).resolve().parent.parent
BINARY = ROOT / "target/debug/sopra-worker"


def screen_wait(terminal: TerminalSession, text: str, timeout: float) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        terminal.read(0.02)
        if text in terminal.screen.text(trim=False):
            return
    raise AssertionError(f"Missing {text!r}:\n{terminal.screen.text()}")


def setup(directory: Path) -> dict[str, str]:
    tools = directory / "tools"
    tools.mkdir()
    for name, source in {
        "slow-cli": '#!/bin/sh\ncase "$1" in\n slow) touch "$PROBE_MARKER"; sleep 5; printf "Options:\\n  --late  Late option\\n" ;;\n *) printf "Usage: slow-cli <command>\\nCommands:\\n  slow  Slow nested command\\n" ;;\nesac\n',
        "fast-cli": '#!/bin/sh\nprintf "Usage: fast-cli <command>\\nCommands:\\n  ready  Ready foreground command\\n"\n',
    }.items():
        path = tools / name
        path.write_text(source)
        path.chmod(0o755)
    shell = directory / "shell"
    shell.mkdir()
    (shell / ".zshrc").write_text(f"HISTFILE=/dev/null\nSAVEHIST=0\nsource '{ROOT}/zsh/editor.zsh'\n")
    config = directory / "config/sopra"
    config.mkdir(parents=True)
    (config / "config.toml").write_text('[prompt]\nleft="$ "\nright=""\n')
    return dict(os.environ, ZDOTDIR=str(shell), XDG_CONFIG_HOME=str(config.parent),
                SOPRA_BIN=str(BINARY), PROBE_MARKER=str(directory / "probe-started"),
                PATH=str(tools) + os.pathsep + os.environ["PATH"])


def interrupt_slow(terminal: TerminalSession, directory: Path) -> None:
    terminal.paste("slow-cli slow ")
    deadline = time.monotonic() + 5
    while not (directory / "probe-started").exists():
        terminal.read(0.02)
        assert time.monotonic() < deadline, terminal.screen.text()
    started = time.monotonic()
    terminal.key("CTRL-C")
    terminal.paste("fast-cli ")
    screen_wait(terminal, "Ready foreground command", 0.8)
    elapsed = time.monotonic() - started
    assert elapsed < 0.8, elapsed
    assert "--late" not in terminal.screen.text()
    print(f"Foreground discovery preemption: {elapsed * 1000:.1f}ms including terminal input/repaint")


def live_builtin(terminal: TerminalSession, directory: Path) -> None:
    subprocess.run(["git", "init", "-q", str(directory)], check=True)
    subprocess.run(["git", "-C", str(directory), "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
                    "commit", "--allow-empty", "-qm", "fixture"], check=True)
    subprocess.run(["git", "-C", str(directory), "branch", "frame-ready-branch"], check=True)
    terminal.key("CTRL-C")
    terminal.paste("git switch frame-ready")
    screen_wait(terminal, "frame-ready-branch", 12)
    terminal.key("TAB")
    screen_wait(terminal, "$ git switch frame-ready-branch", 2)
    terminal.key("CTRL-C")
    (directory / "package.json").write_text(json.dumps({"scripts": {"frame-ready-script": "echo fixture"}}))
    terminal.paste("npm run frame-ready")
    screen_wait(terminal, "frame-ready-script", 12)
    terminal.key("TAB")
    screen_wait(terminal, "$ npm run frame-ready-script", 2)


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="sopra-discovery-terminal-") as temporary:
        directory = Path(temporary)
        env = setup(directory)
        with TerminalSession(("zsh", "-di"), cwd=str(directory), rows=32, columns=140, env=env) as terminal:
            screen_wait(terminal, "$ ", 30)
            interrupt_slow(terminal, directory)
            live_builtin(terminal, directory)
    print("Discovery terminal checks passed (preemption, Git refs, npm scripts)")


if __name__ == "__main__":
    main()
