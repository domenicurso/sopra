"""Extract the published command inventory without depending on Sopra's parser."""
from __future__ import annotations
import re
from completion_audit_cases import Case


def commands(text: str, case: Case) -> set[str]:
    if case.layout == "none":
        return set()
    if case.layout == "usage":
        prefix = rf"^{re.escape(case.command)} {' '.join(case.path)} ([a-z][a-z-]*)"
        return set(re.findall(prefix, text, re.MULTILINE))
    if case.layout == "grid":
        body = text.split("All commands:", 1)[1].lstrip("\n").split("\n\n", 1)[0]
        return set(re.findall(r"[a-z][a-z-]*", body))
    found: set[str] = set()
    active = case.layout in ("git", "groups")
    label_indent: int | None = None
    for line in text.splitlines():
        if line and not line[0].isspace():
            heading = line.rstrip(":").upper()
            if case.layout == "cobra":
                active = heading.endswith("COMMANDS")
            elif case.layout == "table":
                active = heading in ("COMMANDS", "SUBCOMMANDS")
            continue
        if case.path and line.lstrip().startswith(case.command + " "):
            prefix = " ".join((case.command, *case.path)) + " "
            if not line.lstrip().startswith(prefix):
                continue
            line = "  " + line.lstrip()[len(prefix):]
        match = re.match(r"^( +)([a-z][\w-]*)(?:[:,]?[ \t]+)(.*)$", line)
        if not active or not match:
            continue
        indent, name, rest = match.groups()
        if case.layout == "groups" and not re.search(r"\S  +\S", line):
            continue
        if case.layout in ("table", "cobra"):
            label_indent = len(indent) if label_indent is None else min(label_indent, len(indent))
            if len(indent) > label_indent:
                continue
        if rest and not rest.startswith(":"):
            found.add(name)
    return found


def graph_commands(text: str) -> set[str]:
    return set(re.findall(r"^    command (\S+)$", text, re.MULTILINE))


def graph_aliases(text: str) -> set[str]:
    return {alias.strip() for row in re.findall(r"^      aliases: (.+)$", text, re.MULTILINE)
            for alias in row.split(",")}


def flags(text: str) -> set[str]:
    found: set[str] = set()
    for line in text.split("Notes:", 1)[0].splitlines():
        if not re.match(r"^\s{0,8}-[\w?-]", line):
            continue
        # Stop at the description gap; prose mentioning an option is not an option declaration.
        signature = re.split(r"\s{2,}", line.strip(), maxsplit=1)[0]
        found.update(re.findall(r"(?<![\w-])--?[A-Za-z?][\w@-]*", signature))
    return found


def verify(source: str, graph: str, case: Case) -> None:
    assert graph.startswith("command "), "no graph was produced"
    expected = commands(source, case)
    actual = graph_commands(graph)
    aliases = graph_aliases(graph)
    missing, invented = expected - actual - aliases, actual - expected
    assert not missing and not invented, f"commands: missing={sorted(missing)}, invented={sorted(invented)}"
    missing_flags = flags(source) - flags(graph)
    assert not missing_flags, f"missing flags: {sorted(missing_flags)}"
    assert "unknown command or arguments" not in graph, "diagnostics leaked into the graph"
