#!/usr/bin/env python3
"""Check the native ZLE prompt never flashes before the Rust editor paints."""
from __future__ import annotations

import shlex
import subprocess
import tempfile
from pathlib import Path

from terminal_config import terminal_config
from terminal_session import TerminalSession

ROOT = Path(__file__).resolve().parent.parent


def exercise(directory: Path, config_root: str) -> None:
    worker = directory / "slow-worker"
    worker.write_text(
        '#!/bin/sh\nif [ "$1" = --serve ]; then sleep 0.4; fi\n'
        f'exec {shlex.quote(str(ROOT / "target/debug/sopra-worker"))} "$@"\n')
    worker.chmod(0o755)
    (directory / ".zshrc").write_text(
        "PROMPT='NATIVE_STARTUP_FLASH> '\nRPROMPT='NATIVE_RIGHT_FLASH'\n"
        "old_prompt() { PROMPT='NATIVE_PRECMD_FLASH> '; }\n"
        "precmd_functions=(old_prompt)\n"
        f"source '{ROOT}/zsh/editor.zsh'\n")
    environment = {"ZDOTDIR": str(directory), "XDG_CONFIG_HOME": config_root,
                   "SOPRA_BIN": str(worker)}
    with TerminalSession(("zsh", "-di"), rows=16, columns=80,
                         cwd=str(ROOT), env=environment) as terminal:
        terminal.wait_for("ready $ ", timeout=20)
        assert b"NATIVE_" not in terminal.output, bytes(terminal.output)
        terminal.send_text("echo handoff-output")
        terminal.key("ENTER")
        terminal.wait_for("handoff-output", timeout=10)
        terminal.read(1)
        assert "handoff-output" in terminal.screen.lines(), terminal.screen.text()
        assert terminal.screen.lines()[terminal.screen.row] == "ready $", terminal.screen.text()
        assert b"NATIVE_" not in terminal.output, bytes(terminal.output)
        # The transient belongs only to the submitted line, never the next one.
        assert bytes(terminal.output).count(b"submitted>") == 1, bytes(terminal.output)


def main() -> None:
    subprocess.run(["cargo", "build", "--quiet", "--locked", "--bin", "sopra-worker"],
                   cwd=ROOT, check=True)
    config = ("[prompt]\nleft = 'ready $ '\nright = ''\ntransient = 'submitted> '\n"
              "[editor]\ncursor_animation = false\n[completion]\nenabled = false\n")
    with tempfile.TemporaryDirectory(prefix="sopra-prompt-startup-") as temporary:
        with terminal_config(config) as config_root:
            exercise(Path(temporary), config_root)
    print("prompt startup: ok (no native or transient flash during delayed handoff)")


if __name__ == "__main__":
    main()
