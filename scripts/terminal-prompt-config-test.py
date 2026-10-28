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
    (directory / "config.toml").write_text('''[editor]
completion = false
cursor_animation = false
[prompt]
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
    }
    command = ("zsh", "-di")
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
    }
    command = ("zsh", "-di")
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
    (config / "config.toml").write_text("[editor]\ncompletion = false\ncursor_animation = false\n[prompt]\nleft = '{git_status} $ '\nright = ''\n[widgets.git_status]\nformat = '<{items}>'\nitem_format = '[cyan]{item}[/]'\nseparator = ' + '\n")
    subprocess.run(("git", "init", "--quiet", str(directory)), check=True)
    (directory / "staged").write_text("staged")
    (directory / "untracked").write_text("untracked")
    subprocess.run(("git", "-C", str(directory), "add", "staged"), check=True)
    env = {
        "XDG_CONFIG_HOME": str(directory / "config"), "SOPRA_REPO_ROOT": str(root),
        "SOPRA_BIN": str(root / "target/debug/sopra-worker"), "ZDOTDIR": str(root / "examples/isolated"),
    }
    command = ("zsh", "-di")
    with TerminalSession(command, rows=12, columns=100, cwd=str(directory), env=env) as terminal:
        terminal.wait_for("<1! + 2?>", timeout=20)
        assert b"\x1b[36m" in terminal.output, terminal.output


def exercise_saved_multiline(root: Path, config_root: Path) -> None:
    prompt = "[cyan]path[/]\n$ "
    (config_root / "sopra" / "settings").write_text(f"prompt_hex={prompt.encode().hex()}\n")
    script = (
        'source "$SOPRA_REPO_ROOT/zsh/protocol.zsh"; '
        'source "$SOPRA_REPO_ROOT/zsh/settings.zsh"; '
        '_sopra_load_settings 1; print -rn -- "$_SOPRA_CONFIG_PROMPT"'
    )
    environment = dict(os.environ, XDG_CONFIG_HOME=str(config_root), SOPRA_REPO_ROOT=str(root))
    result = subprocess.run(("zsh", "-f", "-c", script), env=environment, text=True, capture_output=True, check=True)
    assert result.stdout == prompt, result.stdout


def exercise_ignored_environment(root: Path, config_root: Path) -> None:
    directory = config_root / "ignored-env" / "sopra"
    directory.mkdir(parents=True)
    (directory / "config.toml").write_text(
        "[prompt]\nleft = 'config $ '\nright = 'configured'\n"
        "[editor]\ncompletion = false\ncursor_animation = false\n"
    )
    environment = {
        "XDG_CONFIG_HOME": str(directory.parent), "SOPRA_REPO_ROOT": str(root),
        "SOPRA_BIN": str(root / "target/debug/sopra-worker"),
        "ZDOTDIR": str(root / "examples/isolated"),
        "SOPRA_PROMPT": "override $ ", "SOPRA_RPROMPT": "wrong",
        "SOPRA_COMPLETION_ENABLED": "1", "SOPRA_CURSOR_ANIMATION": "1",
    }
    with TerminalSession(("zsh", "-di"), rows=20, columns=100, cwd=str(root), env=environment) as terminal:
        terminal.wait_for("configured", timeout=20)
        terminal.read(0.4)
        active = terminal.screen.lines()[terminal.screen.row]
        assert active.startswith("config $ ") and active.endswith("configured"), active
        assert "override" not in active and "wrong" not in active, active
        terminal.send_text("ech")
        terminal.read(0.5)
        assert "0/" not in terminal.screen.text(), terminal.screen.lines()


def exercise_config_precedence(root: Path, config_root: Path) -> None:
    directory = config_root / "precedence" / "sopra"
    directory.mkdir(parents=True)
    (directory / "config.toml").write_text("[prompt]\nleft = 'new'\n[editor]\ncursor_animation = false\n")
    (directory / "prompt.toml").write_text("[prompt]\nleft = 'legacy'\nright = 'fallback'\n")
    (directory / "settings").write_text("cursor_animation=1\nsyntax_highlighting=0\n")
    script = (
        'source "$SOPRA_REPO_ROOT/zsh/protocol.zsh"; source "$SOPRA_REPO_ROOT/zsh/settings.zsh"; '
        '_sopra_load_prompt_config "$(_sopra_config_path)"; '
        '_sopra_load_prompt_config "${$(_sopra_config_path):h}/prompt.toml"; '
        '_sopra_load_settings; print -r -- "$_SOPRA_CONFIG_PROMPT|$_SOPRA_CONFIG_RPROMPT|$_SOPRA_CONFIG_CURSOR_ANIMATION|$_SOPRA_CONFIG_SYNTAX_HIGHLIGHTING"'
    )
    env = {"PATH": os.environ["PATH"], "HOME": os.environ["HOME"], "XDG_CONFIG_HOME": str(directory.parent),
           "SOPRA_REPO_ROOT": str(root), "SOPRA_BIN": str(root / "target/debug/sopra-worker"),
           "SOPRA_PROMPT": "environment", "SOPRA_RPROMPT": "environment",
           "SOPRA_CURSOR_ANIMATION": "1", "SOPRA_SYNTAX_HIGHLIGHTING": "1"}
    result = subprocess.run(("zsh", "-f", "-c", script), env=env, text=True, capture_output=True, check=True)
    assert result.stdout.strip() == "new|fallback|0|0", result.stdout



def exercise_config_only_command(root: Path) -> None:
    worker = root / "target/debug/sopra-worker"
    help_result = subprocess.run((str(worker), "--help"), text=True, capture_output=True, check=True)
    assert "config.toml" in help_result.stdout and "settings" not in help_result.stdout
    removed = subprocess.run((str(worker), "--settings"), text=True, capture_output=True)
    assert removed.returncode != 0 and "unknown argument" in removed.stderr


def main() -> None:
    root = Path(__file__).resolve().parent.parent
    subprocess.run(["cargo", "build", "--quiet", "--locked", "--bin", "sopra-worker"], cwd=root, check=True)
    with tempfile.TemporaryDirectory(prefix="sopra-prompt-config-") as temporary:
        exercise_default_colors(root, Path(temporary))
        exercise_config(root, Path(temporary))
        exercise_ignored_environment(root, Path(temporary))
        exercise_saved_multiline(root, Path(temporary))
        exercise_git_format(root, Path(temporary))
        exercise_config_precedence(root, Path(temporary))
        exercise_config_only_command(root)
    print("prompt config: ok (TOML, status, duration, transient prompt)")


if __name__ == "__main__":
    main()
