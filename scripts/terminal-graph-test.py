#!/usr/bin/env python3
"""Check the public graph debugger, offline parsing, and binary cache recovery."""
from __future__ import annotations
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
BINARY = ROOT / "target/debug/sopra-worker"


def run(arguments: list[str], env: dict[str, str], source: str | None = None) -> str:
    result = subprocess.run([str(BINARY), "debug", "graph", *arguments], input=source,
                            env=env, capture_output=True, text=True, timeout=25)
    assert result.returncode == 0, (arguments, result.stdout, result.stderr)
    return result.stdout


def fixture(directory: Path) -> tuple[Path, dict[str, str]]:
    source = directory / "help.txt"
    source.write_text("Usage: demo-cli <command>\n\nCommands:\n  build  Build fixtures\n\nOptions:\n  --help-extra  Display extended help\n  --before      Before refresh\n")
    program = directory / "demo-cli"
    program.write_text('''#!/bin/sh
printf '%s\\n' "$*" >> "$GRAPH_PROBES"
case "$1" in
  --help) cat "$GRAPH_HELP" ;;
  --help-extra) printf 'Options:\\n  -g:{lines,vars,source}  Select debug information\\n' ;;
  *) printf 'demo-cli: unknown arguments\\n' >&2; exit 1 ;;
esac
''')
    program.chmod(0o755)
    env = dict(os.environ, TMPDIR=str(directory), GRAPH_HELP=str(source),
               GRAPH_PROBES=str(directory / "probes"))
    return program, env


def cache_recovery(program: Path, env: dict[str, str]) -> None:
    args = ["--root", str(program)]
    graph = run(["generate", *args], env)
    assert "command build" in graph and "-g:" in graph, graph
    assert "choices=lines|vars|source" in graph, graph
    probes = Path(env["GRAPH_PROBES"])
    before = probes.read_text()
    assert "Cache: hit" in run(["sources", *args], env)
    assert probes.read_text() == before, "warm source inspection ran probes"
    report = run(["cache", *args], env)
    assert "State: valid" in report, report
    cache_file = Path(report.split("Cache file: ", 1)[1].splitlines()[0])
    assert cache_file.read_bytes().startswith(b"SOPRACCH")
    cache_file.write_bytes(b"broken")
    assert "invalid binary bundle" in run(["cache", *args], env)
    assert run(["generate", *args], env) == graph
    assert probes.read_text() != before, "corrupt cache did not trigger new probes"
    assert "State: valid" in run(["cache", *args], env)


def refresh(program: Path, env: dict[str, str]) -> None:
    source = Path(env["GRAPH_HELP"])
    source.write_text(source.read_text().replace("--before", "--after"))
    args = ["--root", str(program)]
    assert "--before" in run(["generate", *args], env)
    graph = run(["generate", "--fresh", *args], env)
    assert "--after" in graph and "--before" not in graph, graph
    assert run(["generate", *args], env) == graph


def offline_parsing(directory: Path, env: dict[str, str]) -> None:
    probes = Path(env["GRAPH_PROBES"])
    before = probes.read_text()
    source = Path(env["GRAPH_HELP"])
    graph = run(["parse", "uninstalled-command", str(source)], env)
    assert "command build" in graph and "command uninstalled-command" in graph, graph
    assert run(["parse", "uninstalled-command", "-"], env, source.read_text()) == graph
    manual = directory / "manual.1"
    manual.write_text('.TH DEMO 1\n.SH OPTIONS\n.TP\n.B --verbose\nPrint detail\n')
    assert "--verbose" in run(["parse", "--man", "uninstalled-command", str(manual)], env)
    assert probes.read_text() == before, "offline parsing executed the target command"
    result = subprocess.run([str(BINARY), "debug", "graph", "parse", "demo", "-"],
                            input="demo: unknown command", text=True, env=env, capture_output=True)
    assert result.returncode != 0 and "no recognized completion data" in result.stderr


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="sopra-graph-test-") as temporary:
        directory = Path(temporary)
        program, env = fixture(directory)
        cache_recovery(program, env)
        refresh(program, env)
        offline_parsing(directory, env)
    print("Graph debugger and cache recovery checks passed")


if __name__ == "__main__":
    main()
