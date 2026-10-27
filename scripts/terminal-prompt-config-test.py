#!/usr/bin/env python3
"""Exercise TOML prompt configuration and shell context in an interactive Zsh."""
from __future__ import annotations

import os
import re
import subprocess
import tempfile
from pathlib import Path

from terminal_session import TerminalSession


def exercise_config(root: Path, config_root: Path) -> None:
    directory = config_root / "sopra"
    directory.mkdir()
    (directory / "prompt.toml").write_text('''[prompt]
left = "[cyan]{user}[/] $ "
right = "[if status]FAIL={status} [/][if duration]TIME={duration} [/]{clock}"
transient = "done> "
[widgets.clock]
format = "%H[dim]:[/]%M"
''')
    shell_dir = config_root / "zsh"
    shell_dir.mkdir()
    (shell_dir / ".zshrc").write_text("record_status() { print -r -- $? >> $HOOK_LOG; }\nprecmd_functions=(record_status)\nsource $SOPRA_REPO_ROOT/zsh/editor.zsh\n")
    environment = {
        "HOOK_LOG": str(config_root / "hook.log"),
        "XDG_CONFIG_HOME": str(config_root), "SOPRA_REPO_ROOT": str(root),
        "SOPRA_BIN": str(root / "target/debug/sopra-worker"),
        "ZDOTDIR": str(shell_dir),
        "SOPRA_COMPLETION_ENABLED": "0", "SOPRA_CURSOR_ANIMATION": "0",
    }
    command = ("env", "-u", "SOPRA_PROMPT", "-u", "SOPRA_RPROMPT", "-u", "SOPRA_CLOCK_FORMAT", "-u", "SOPRA_TRANSIENT_PROMPT", "zsh", "-di")
    with TerminalSession(command, rows=20, columns=100, cwd=str(root), env=environment) as terminal:
        terminal.wait_for(" $ ", timeout=20)
        terminal.read(0.4)
        assert re.search(r"\d\d:\d\d", terminal.screen.text()), terminal.screen.lines()
        terminal.send_text("false")
        terminal.key("ENTER")
        terminal.wait_for("FAIL=1", timeout=10)
        terminal.read(0.3)
        assert (config_root / "hook.log").read_text().splitlines()[-1] == "1"
        assert "TIME=" in terminal.screen.text(), terminal.screen.lines()
        terminal.send_text("sleep 0.2")
        terminal.key("ENTER")
        terminal.read(1)
        active = terminal.screen.lines()[terminal.screen.row]
        assert "FAIL=" not in active and re.search(r"TIME=\d+ms", active), active
        assert "done> sleep 0.2" in terminal.screen.text(), terminal.screen.lines()


def exercise_default_colors(root: Path, config_root: Path) -> None:
    environment = {
        "XDG_CONFIG_HOME": str(config_root), "SOPRA_REPO_ROOT": str(root),
        "SOPRA_BIN": str(root / "target/debug/sopra-worker"),
        "ZDOTDIR": str(root / "examples/isolated"),
        "SOPRA_COMPLETION_ENABLED": "0", "SOPRA_CURSOR_ANIMATION": "0",
    }
    command = ("env", "-u", "SOPRA_PROMPT", "-u", "SOPRA_RPROMPT", "zsh", "-di")
    with TerminalSession(command, rows=20, columns=120, cwd=str(root), env=environment) as terminal:
        terminal.wait_for(" in ", timeout=20)
        terminal.read(0.5)
        output = bytes(terminal.output)
        assert b"\x1b[31m" in output and b"\x1b[36m" in output, output
        assert b"\x1b[1m\x1b[36m" in output and b"\x1b[2m" in output, output
        assert " on " in terminal.screen.text(), terminal.screen.lines()


def exercise_git_format(root: Path, config_root: Path) -> None:
    directory = config_root / "custom-git"
    directory.mkdir()
    config = directory / "config" / "sopra"
    config.mkdir(parents=True)
    (config / "prompt.toml").write_text("[prompt]\nleft = '{git_status} $ '\n[widgets.git_status]\nformat = '<{items}>'\nitem_format = '[cyan]{item}[/]'\nseparator = ' + '\n")
    subprocess.run(("git", "init", "--quiet", str(directory)), check=True)
    (directory / "staged").write_text("staged")
    (directory / "untracked").write_text("untracked")
    subprocess.run(("git", "-C", str(directory), "add", "staged"), check=True)
    env = {
        "XDG_CONFIG_HOME": str(directory / "config"), "SOPRA_REPO_ROOT": str(root),
        "SOPRA_BIN": str(root / "target/debug/sopra-worker"), "ZDOTDIR": str(root / "examples/isolated"),
        "SOPRA_COMPLETION_ENABLED": "0", "SOPRA_CURSOR_ANIMATION": "0", "SOPRA_RPROMPT": "",
    }
    command = ("env", "-u", "SOPRA_PROMPT", "-u", "SOPRA_GIT_STATUS_FORMAT", "-u", "SOPRA_GIT_STATUS_SEPARATOR", "-u", "SOPRA_GIT_STATUS_ITEM_FORMAT", "zsh", "-di")
    with TerminalSession(command, rows=12, columns=100, cwd=str(directory), env=env) as terminal:
        terminal.wait_for("<1! + 2?>", timeout=20)
        assert b"\x1b[36m" in terminal.output, terminal.output


def exercise_saved_multiline(root: Path, config_root: Path) -> None:
    prompt = "[cyan]path[/]\n$ "
    (config_root / "sopra" / "settings").write_text(f"prompt_hex={prompt.encode().hex()}\n")
    script = (
        'source "$SOPRA_REPO_ROOT/zsh/protocol.zsh"; '
        'source "$SOPRA_REPO_ROOT/zsh/settings.zsh"; '
        '_sopra_load_settings 1; print -rn -- "$SOPRA_PROMPT"'
    )
    environment = dict(os.environ, XDG_CONFIG_HOME=str(config_root), SOPRA_REPO_ROOT=str(root))
    result = subprocess.run(("zsh", "-f", "-c", script), env=environment, text=True, capture_output=True, check=True)
    assert result.stdout == prompt, result.stdout


def exercise_environment_override(root: Path, config_root: Path) -> None:
    environment = {
        "XDG_CONFIG_HOME": str(config_root), "SOPRA_REPO_ROOT": str(root),
        "SOPRA_BIN": str(root / "target/debug/sopra-worker"),
        "ZDOTDIR": str(root / "examples/isolated"),
        "SOPRA_PROMPT": "override $ ", "SOPRA_RPROMPT": "",
        "SOPRA_COMPLETION_ENABLED": "0", "SOPRA_CURSOR_ANIMATION": "0",
    }
    with TerminalSession(("zsh", "-di"), rows=20, columns=100, cwd=str(root), env=environment) as terminal:
        terminal.wait_for("override $ ", timeout=20)
        terminal.read(0.5)
        active = terminal.screen.lines()[terminal.screen.row]
        assert active.strip() == "override $", active


def main() -> None:
    root = Path(__file__).resolve().parent.parent
    subprocess.run(["cargo", "build", "--quiet", "--locked", "--bin", "sopra-worker"], cwd=root, check=True)
    with tempfile.TemporaryDirectory(prefix="sopra-prompt-config-") as temporary:
        exercise_default_colors(root, Path(temporary))
        exercise_config(root, Path(temporary))
        exercise_environment_override(root, Path(temporary))
        exercise_saved_multiline(root, Path(temporary))
        exercise_git_format(root, Path(temporary))
    print("prompt config: ok (TOML, status, duration, transient prompt)")


if __name__ == "__main__":
    main()
