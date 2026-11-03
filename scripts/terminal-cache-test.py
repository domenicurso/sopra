#!/usr/bin/env python3
"""Verify cache controls against disk data and a running editor in a private temp store."""
from __future__ import annotations
import os
from pathlib import Path
import shlex
import socket
import subprocess
import tempfile
import threading
import time
from terminal_session import TerminalSession

ROOT = Path(__file__).resolve().parent.parent
BINARY = ROOT / "target/debug/sopra-worker"


def run(args: list[str], directory: Path, env: dict[str, str]) -> str:
    result = subprocess.run([str(BINARY), *args], cwd=directory, env=env,
                            capture_output=True, text=True, timeout=15)
    assert result.returncode == 0, (args, result.stdout, result.stderr)
    return result.stdout


def wait_screen(terminal: TerminalSession, text: str) -> None:
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        terminal.read(0.05)
        if text in terminal.screen.text(trim=False):
            return
    raise AssertionError((text, terminal.screen.text()))


def submit(terminal: TerminalSession, command: str, directory: Path) -> str:
    result, status = directory / "result", directory / "status"
    result.unlink(missing_ok=True)
    status.unlink(missing_ok=True)
    terminal.key("CTRL-C")
    terminal.paste(f"{command} > {shlex.quote(str(result))} 2>&1; print $? > {shlex.quote(str(status))}")
    terminal.key("ENTER")
    deadline = time.monotonic() + 10
    while not status.exists() and time.monotonic() < deadline:
        terminal.read(0.05)
    assert status.exists(), terminal.screen.text()
    assert status.read_text().strip() == "0", result.read_text()
    terminal.read(0.1)
    return result.read_text()


def setup(directory: Path) -> dict[str, str]:
    tools, shell, config = directory / "tools", directory / "shell", directory / "config/sopra"
    for path in [tools, shell, config, directory / "tmp"]:
        path.mkdir(parents=True)
    (directory / "schema").write_text("cached")
    for name in ["cache-cli", "keep-cli"]:
        tool = tools / name
        tool.write_text('#!/bin/sh\nprintf "Options:\\n  --%s  Schema from fixture\\n" "$(cat "$SCHEMA")"\n')
        tool.chmod(0o755)
    (config / "config.toml").write_text('[prompt]\nleft="$ "\nright=""\n[editor]\nautosuggestions=false\n')
    (shell / ".zshrc").write_text(f"HISTFILE=/dev/null\nSAVEHIST=0\nsource '{ROOT}/zsh/editor.zsh'\n")
    return dict(os.environ, _SOPRA_SERVER_SOCKET="", ZDOTDIR=str(shell), TMPDIR=str(directory / "tmp"),
                XDG_CONFIG_HOME=str(config.parent), SOPRA_BIN=str(BINARY), SCHEMA=str(directory / "schema"),
                PATH=str(tools) + os.pathsep + os.environ["PATH"])


def older_worker(directory: Path, env: dict[str, str]) -> None:
    address = directory / "old-worker"
    with socket.socket(socket.AF_UNIX) as listener:
        listener.bind(str(address))
        listener.listen(1)

        def respond() -> None:
            connection, _ = listener.accept()
            with connection:
                connection.recv(4096)
                body = b"unknown worker control request"
                connection.sendall(f"C1\terror\t{len(body)}\n".encode() + body)

        worker = threading.Thread(target=respond, daemon=True)
        worker.start()
        result = subprocess.run([str(BINARY), "cache", "status"], env={**env, "_SOPRA_SERVER_SOCKET": str(address)},
                                capture_output=True, text=True, timeout=5)
        worker.join(timeout=5)
        assert result.returncode != 0 and "sopra restart" in result.stderr, result.stderr


def exercise(directory: Path, env: dict[str, str]) -> None:
    assert "Sources: 0 files" in run(["cache", "status"], directory, env)
    help_text = run(["--help"], directory, env)
    assert "config <command>" in help_text and "config edit" not in help_text
    for child in ["edit", "check", "path", "show"]:
        assert child in run(["config", "--help"], directory, env)
    for name in ["cache-cli", "keep-cli"]:
        run(["debug", "graph", "generate", "--root", name], directory, env)
    assert "Graphs: 4 files" in run(["cache", "status"], directory, env)
    with TerminalSession(("zsh", "-di"), cwd=str(directory), rows=30, columns=140, env=env) as terminal:
        wait_screen(terminal, "$ ")
        terminal.paste("sopra cache cle")
        wait_screen(terminal, "Clear all completion data")
        started = time.monotonic()
        terminal.key("TAB")
        wait_screen(terminal, "$ sopra cache clear ")
        terminal.send_text("cache-cli")
        wait_screen(terminal, "$ sopra cache clear cache-cli")
        assert time.monotonic() - started < 0.8, "Tab and subsequent typing stalled"
        terminal.key("CTRL-C")
        terminal.paste("cache-cli --")
        wait_screen(terminal, "--cached")
        socket = submit(terminal, 'print -r -- "$_SOPRA_SERVER_SOCKET"', directory).strip()
        terminal.paste("echo unsubmitted")
        busy = subprocess.run([str(BINARY), "cache", "clear"], env={**env, "_SOPRA_SERVER_SOCKET": socket},
                              capture_output=True, text=True, timeout=5)
        assert busy.returncode != 0 and "worker is editing" in busy.stderr, busy.stderr
        (directory / "schema").write_text("fresh")
        report = submit(terminal, "sopra cache clear cache-cli", directory)
        assert "Worker completion memory reset" in report, report
        assert "--cached" in run(["debug", "graph", "generate", "--root", "keep-cli"], directory, env)
        terminal.paste("cache-cli --")
        wait_screen(terminal, "--fresh")
        assert "Worker completion memory reset" in submit(terminal, "sopra cache clear", directory)
        report = run(["cache", "status"], directory, env)
        assert "Sources: 0 files" in report and "Graphs: 0 files" in report, report
        report = submit(terminal, "sopra cache status", directory)
        assert "Sources:" in report and "Graphs:" in report, report
    run(["cache", "clear", "absent-cli"], directory, env)
    run(["cache", "clear"], directory, env)
    assert "Graphs: 0 files" in run(["cache", "status"], directory, env)
    for store in ["completion-cache", "completion-graphs"]:
        assert not (Path(env["TMPDIR"]) / "sopra" / store).exists()


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="sopra-cache-", dir="/tmp") as temporary:
        directory = Path(temporary)
        env = setup(directory)
        older_worker(directory, env)
        exercise(directory, env)
    print("Cache controls: ok (status, scopes, memory invalidation, busy worker, grouped config help)")


if __name__ == "__main__":
    main()
