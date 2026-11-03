"""Check alias grouping and explicitly enumerated choices against declaration rows."""
from __future__ import annotations
import re
from completion_audit_cases import Case


def verify_fields(source: str, graph: dict, case: Case) -> None:
    options = graph["options"]
    for line in source.split("Notes:", 1)[0].splitlines():
        if not re.match(r"^\s{0,8}-[\w?-]", line):
            continue
        parts = re.split(r"\s{2,}", line.strip(), maxsplit=1)
        declaration = parts[0]
        names = re.findall(r"(?<![\w-])--?[A-Za-z?][\w@.-]*", declaration)
        # Help rows can declare independent modes separated by `|`, rather than aliases.
        if len(names) < 2 or "," not in declaration or "|" in declaration or "=" in declaration or ": " in declaration:
            continue
        matching = [option for option in options if any(name in option["names"] for name in names)]
        if not matching or not all(any(name in option["names"] for option in matching) for name in names):
            continue  # Inventory validation reports unsupported option names separately.
        assert any(set(names) <= set(option["names"]) for option in matching), (case, "aliases split", names)
        if len(parts) == 2:
            choices = re.search(r"\[(?:possible values|choices):\s*([^]]+)\]", parts[1], re.IGNORECASE)
            if choices:
                expected = {value.strip().strip("'\"") for value in choices[1].split(",")}
                actual = {choice for option in matching if option["value"] for choice in option["value"]["choices"]}
                assert expected <= actual, (case, "missing choices", expected - actual)
