#!/usr/bin/env python3
"""Check developer checkout linking, workflow processes, and debug CLI output."""
from __future__ import annotations

import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
BINARY = ROOT / "target/debug/sopra-worker"


def run(args: list[str], env: dict[str, str], cwd: Path, success: bool = True) -> str:
    result = subprocess.run([str(BINARY), *args], cwd=cwd, env=env,
                            capture_output=True, text=True, timeout=30)
    assert (result.returncode == 0) == success, (args, result.stdout, result.stderr)
    return result.stdout + result.stderr


def checkout(directory: Path) -> None:
    (directory / "scripts").mkdir(parents=True)
    (directory / "zsh").mkdir()
    (directory / "Cargo.toml").write_text('[package]\nname="sopra"\n')
    (directory / "zsh/editor.zsh").write_text("")
    for name, label in [("build.sh", "build"), ("start.sh", "shell")]:
        script = directory / "scripts" / name
        script.write_text(f'#!/bin/sh\nprintf "%s\\n" "{label}" "$PWD" >> "$DEV_LOG"\n')
        script.chmod(0o755)


def environment(directory: Path) -> dict[str, str]:
    env = {key: value for key, value in os.environ.items()
           if key not in ("SOPRA_REPO_ROOT", "_SOPRA_SERVER_SOCKET")}
    env.update(XDG_CONFIG_HOME=str(directory / "config"), XDG_CACHE_HOME=str(directory / "cache"),
               DEV_LOG=str(directory / "log"))
    tools = directory / "tools"
    tools.mkdir()
    cargo = tools / "cargo"
    cargo.write_text('#!/bin/sh\nprintf "%s\\n" cargo "$PWD" "$@" >> "$DEV_LOG"\n'
                     'exit "${DEV_EXIT:-0}"\n')
    cargo.chmod(0o755)
    env["PATH"] = str(tools) + os.pathsep + env.get("PATH", "")
    return env


def links(directory: Path, env: dict[str, str], linked: Path) -> None:
    assert run(["dev", "path"], env, ROOT / "src").strip() == str(ROOT)
    assert run(["dev", "path"], env, directory).strip() == str(ROOT)
    run(["dev", "link", str(linked)], env, directory)
    assert run(["dev", "path"], env, directory).strip() == str(linked)
    assert run(["dev", "path"], {**env, "SOPRA_REPO_ROOT": str(ROOT)}, directory).strip() == str(linked)
    run(["dev", "link", str(directory)], env, directory, False)
    assert run(["dev", "path"], env, directory).strip() == str(linked)
    run(["dev", "unlink"], env, directory)
    run(["dev", "unlink"], env, directory)
    assert run(["dev", "path"], env, directory).strip() == str(ROOT)
    run(["dev", "link", str(linked)], env, directory)


def workflows(directory: Path, env: dict[str, str], linked: Path) -> None:
    log = Path(env["DEV_LOG"])
    for command in ["build", "shell"]:
        run(["dev", command], env, directory)
        assert log.read_text().splitlines() == [command, str(linked)]
        log.unlink()
    run(["dev", "test", "a filter", "--", "--nocapture"], env, directory)
    assert log.read_text().splitlines() == ["cargo", str(linked), "test", "--workspace",
                                           "--locked", "a filter", "--", "--nocapture"]
    log.unlink()
    run(["dev", "check"], env, directory)
    assert log.read_text().splitlines() == ["cargo", str(linked), "fmt", "--all", "--", "--check",
                                           "cargo", str(linked), "clippy", "--workspace",
                                           "--all-targets", "--locked", "--", "-D", "warnings"]
    log.unlink()
    output = run(["dev", "check"], {**env, "DEV_EXIT": "7"}, directory, False)
    assert "7" in output and "clippy" not in log.read_text()
    log.unlink()
    (linked / "scripts/build.sh").unlink()
    assert "not a Sopra source checkout" in run(["dev", "path"], env, directory, False)
    run(["dev", "unlink"], env, directory)


def debug_commands(directory: Path, env: dict[str, str]) -> None:
    report = run(["debug", "syntax", 'if echo -n "$HOME" | cat; then /no-such-sopra-command; fi'], env, directory)
    assert "Keyword\tYellow" in report and "Flag\tBlue" in report and "FakeCommand\tRed" in report
    marker = directory / "not-executed"
    run(["debug", "syntax", f"touch {marker}"], env, directory)
    assert not marker.exists()
    tool = directory / "demo-cli"
    tool.write_text('#!/bin/sh\nif [ "$1" = access ]; then\n'
                    "printf 'Usage: demo-cli access [options]\\nOptions:\\n  --permission VALUE  Choose permissions\\n'\n"
                    "else\nprintf 'Usage: demo-cli <command> [options]\\nCommands:\\n  access  Manage access\\nOptions:\\n  --verbose  Show details\\n'\nfi\n")
    tool.chmod(0o755)
    for mode in [[], ["--root"], ["--chunk"]]:
        args = ["debug", "completions", *mode, str(tool)]
        if mode == ["--chunk"]:
            args.append("access")
        graph = run([*args, "--cwd", str(directory)], env, directory)
        expected = "permission" if mode == ["--chunk"] else "access"
        assert expected in graph, graph


def arguments(directory: Path, env: dict[str, str]) -> None:
    for group in ["dev", "debug"]:
        for suffix in [[], ["--help"], ["-h"]]:
            assert f"Usage: sopra {group}" in run([group, *suffix], env, directory)
    for args in [["dev", "build", "extra"], ["dev", "link"], ["dev", "path", "extra"],
                 ["dev", "unknown"], ["debug", "syntax"], ["debug", "syntax", "one", "two"],
                 ["debug", "completions"], ["debug", "unknown"]]:
        run(args, env, directory, False)


def main() -> None:
    subprocess.run([str(ROOT / "scripts/build.sh")], cwd=ROOT, check=True)
    with tempfile.TemporaryDirectory(prefix="sopra-dev-") as temporary:
        directory = Path(temporary).resolve()
        env = environment(directory)
        linked = directory / "linked checkout"
        checkout(linked)
        links(directory, env, linked)
        workflows(directory, env, linked)
        debug_commands(directory, env)
        arguments(directory, env)
    print("developer CLI: ok (linking, workflows, failures, syntax, completion graphs)")


if __name__ == "__main__":
    main()
