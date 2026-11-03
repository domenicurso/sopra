#!/usr/bin/env python3
"""Compare installed CLI help inventories with generated graphs; save evidence outside the repo."""
from __future__ import annotations
import argparse
import json
from concurrent.futures import ThreadPoolExecutor
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
from completion_audit_cases import CASES, Case
from completion_audit_sources import verify
from completion_audit_semantics import verify_fields

ROOT = Path(__file__).resolve().parent.parent
BINARY = ROOT / "target/debug/sopra-worker"


def capture(command: list[str], directory: Path) -> str:
    env = dict(os.environ, TERM="dumb", NO_COLOR="1", LC_ALL="C", PAGER="cat", MANPAGER="cat")
    result = subprocess.run(command, cwd=directory, env=env, capture_output=True, text=True, timeout=45)
    return result.stdout + result.stderr


def audit(case: Case, output: Path) -> tuple[str, str]:
    label = "-".join((case.command, *case.path))
    executable = str(BINARY) if case.command == "sopra" else shutil.which(case.command)
    if executable is None:
        return label, "SKIP (not installed)"
    directory = output / label
    directory.mkdir()
    try:
        source = capture([executable, *case.path, "--help"], directory)
        (directory / "help.txt").write_text(source)
        for flag in case.extra_help:
            extra = capture([executable, *case.path, flag], directory)
            (directory / (flag.lstrip("-") + ".txt")).write_text(extra)
            source += "\n" + extra
        mode = "--chunk" if case.path else "--root"
        command = [str(BINARY), "debug", "completions", mode, executable, *case.path]
        graph = capture(command, directory)
        (directory / "graph.txt").write_text(graph)
        verify(source, graph, case)
        structured = capture([str(BINARY), "debug", "graph", "json", mode, executable, *case.path], directory)
        (directory / "graph.json").write_text(structured)
        verify_fields(source, json.loads(structured), case)
        assert capture(command, directory) == graph, "warm cache changed the graph"
    except (AssertionError, OSError, ValueError, subprocess.TimeoutExpired) as error:
        return label, f"FAIL {error}"
    return label, "PASS"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, help="new directory for source and graph evidence")
    args = parser.parse_args()
    output = args.output or Path(tempfile.mkdtemp(prefix="sopra-completion-audit-"))
    output.mkdir(parents=True, exist_ok=True)
    with ThreadPoolExecutor(max_workers=4) as pool:
        results = list(pool.map(lambda case: audit(case, output), CASES))
    for label, result in results:
        print(f"{label}: {result}")
    print(f"Evidence: {output}")
    raise SystemExit(any(result.startswith("FAIL") for _, result in results))


if __name__ == "__main__":
    main()
