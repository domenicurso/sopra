"""Installed CLI corpus for the source/graph audit, independent of parser internals."""
from __future__ import annotations
from dataclasses import dataclass


@dataclass(frozen=True)
class Case:
    command: str
    path: tuple[str, ...] = ()
    layout: str = "none"
    extra_help: tuple[str, ...] = ()


CASES = (
    Case("sopra", layout="table"), Case("sopra", ("dev",), "table"),
    Case("sopra", ("debug",), "table"), Case("sopra", ("config",), "table"),
    Case("sopra", ("widgets",), "table"), Case("sopra", ("debug", "completions")),
    Case("sopra", ("debug", "graph"), "table"),
    Case("git", layout="git"),
    Case("git", ("clone",)), Case("git", ("diff",)),
    Case("cargo", layout="table"), Case("cargo", ("new",)),
    Case("cargo", ("build",)), Case("rustup", layout="table"),
    Case("rustup", ("toolchain",), "table"), Case("rg"), Case("bat"),
    Case("gh", layout="cobra"), Case("gh", ("repo",), "cobra"),
    Case("gh", ("pr",), "cobra"), Case("gh", ("repo", "clone")),
    Case("npm", layout="grid"), Case("npm", ("access",), "usage"),
    Case("bun", layout="table"), Case("bun", ("pm",), "table"),
    Case("deno", layout="table"), Case("deno", ("task",)),
    Case("pnpm", layout="groups"), Case("pnpm", ("store",), "table"),
    Case("javac", extra_help=("--help-extra",)), Case("java"), Case("jar"),
    Case("curl"), Case("tar"), Case("python3"),
)
