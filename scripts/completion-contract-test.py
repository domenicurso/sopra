#!/usr/bin/env python3
"""Check graph fields against a maintained source oracle and execute accepted insertions."""
from __future__ import annotations
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
BINARY = ROOT / "target/debug/sopra-worker"
FIXTURES = ROOT / "scripts/fixtures/completion"


def debug(arguments: list[str], directory: Path, env: dict[str, str]) -> str:
    result = subprocess.run([str(BINARY), "debug", *arguments], cwd=directory, env=env,
                            capture_output=True, text=True, timeout=20)
    assert result.returncode == 0, result.stderr
    return result.stdout


def setup(directory: Path) -> dict[str, str]:
    executable = directory / "contract-cli"
    source = FIXTURES / "contract.help"
    executable.write_text(f'#!/usr/bin/env python3\nimport sys,json\nfrom pathlib import Path\n'
                          f'if any(arg in ("--help","-h") for arg in sys.argv[1:]): print(Path({str(source)!r}).read_text())\n'
                          'else: print(json.dumps(sys.argv[1:]))\n')
    executable.chmod(0o755)
    config = directory / "config/sopra"
    config.mkdir(parents=True)
    (config / "completions.toml").write_text('[[provider]]\nid="contract-values"\ncommand="contract-cli"\n'
        'option="--tag"\njson_file="values.json"\npointer="/values"\nroot_marker="values.json"\nwatch=["values.json"]\n')
    (directory / "values.json").write_text(json.dumps({"values": {"space value": "", "semi;$(bad)": "", "π-value": ""}}))
    (directory / "file space.txt").write_text("fixture")
    (directory / "a-directory").mkdir()
    return dict(os.environ, XDG_CONFIG_HOME=str(config.parent), PATH=str(directory) + os.pathsep + os.environ["PATH"])


def verify_graph(directory: Path, env: dict[str, str]) -> None:
    graph = json.loads(debug(["graph", "json", "--root", "contract-cli"], directory, env))
    for name, expected in json.loads((FIXTURES / "contract.expected.json").read_text()).items():
        option = next(option for option in graph["options"] if name in option["names"])
        for field, value in expected.items():
            actual = option.get(field, option["value"].get(field) if option["value"] else None)
            assert actual == value, (name, field, actual, value)
    assert ["--json", "--plain"] in graph["exclusive"], graph
    explanation = debug(["graph", "explain", "--root", "contract-cli", "--", "--mode"], directory, env)
    assert "help source 1:" in explanation and "Choose execution mode" in explanation, explanation
    assert "attachment=Separate" in explanation, explanation


def candidate(line: str, display: str, directory: Path, env: dict[str, str], cursor: int | None = None) -> dict:
    args = ["candidates", line]
    if cursor is not None:
        args += ["--cursor", str(cursor)]
    result = json.loads(debug(args, directory, env))
    return next(item for item in result["items"] if item["display"] == display)


def accepted(line: str, item: dict, directory: Path, env: dict[str, str]) -> list[str]:
    data = line.encode()
    replaced = (data[:item["replace"]["start"]] + item["insert"].encode() + data[item["replace"]["end"]:]).decode()
    result = subprocess.run(["zsh", "-f", "-c", replaced], cwd=directory, env=env,
                            capture_output=True, text=True, check=True, timeout=5)
    return json.loads(result.stdout)


def verify_insertions(directory: Path, env: dict[str, str]) -> None:
    for line, display, expected in [
        ("contract-cli --mode sl", "slow", ["--mode", "slow"]),
        ("contract-cli --color=ne", "never", ["--color=never"]),
        ("contract-cli --tag sp", "space value", ["--tag", "space value"]),
        ("contract-cli --tag 'sp", "space value", ["--tag", "space value"]),
        ('contract-cli --tag "semi', "semi;$(bad)", ["--tag", "semi;$(bad)"]),
        ("contract-cli --tag π", "π-value", ["--tag", "π-value"]),
    ]:
        assert accepted(line, candidate(line, display, directory, env), directory, env) == expected, line
    line = "contract-cli --mode sl --json"
    item = candidate(line, "slow", directory, env, cursor=len("contract-cli --mode sl"))
    assert accepted(line, item, directory, env) == ["--mode", "slow", "--json"]
    line = "contract-cli --output=fi"
    data = json.loads(debug(["candidates", line], directory, env))
    item = next(item for item in data["items"] if "file" in item["display"])
    assert accepted(line, item, directory, env) == ["--output=file space.txt"], item
    directories = json.loads(debug(["candidates", "contract-cli --directory "], directory, env))["items"]
    assert all(item["kind"] == "Directory" for item in directories), directories


def verify_container_recipe(directory: Path, env: dict[str, str]) -> None:
    executable = directory / "docker"
    executable.write_text('#!/bin/sh\ncase "$1" in\n ps) printf "fixture-container\\n" ;;\n start|logs) printf "Usage: docker %s <container>\\n" "$1" ;;\n *) printf "Usage: docker <command>\\nCommands:\\n  start  Start containers\\n  logs   Show logs\\n" ;;\nesac\n')
    executable.chmod(0o755)
    for command in ("start", "logs"):
        line = f"docker {command} fixture"
        item = candidate(line, "fixture-container", directory, env)
        assert item["source"] == "ShellContext", item
        assert item["insert"] == "fixture-container", item


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="sopra-contract-") as temporary:
        directory = Path(temporary)
        env = setup(directory)
        verify_graph(directory, env)
        verify_insertions(directory, env)
        verify_container_recipe(directory, env)
    print("Completion contracts: ok (source fields, provenance, quoting, Unicode, ranges, accepted argv)")


if __name__ == "__main__":
    main()
