"""Temporary Sopra configuration for terminal sessions."""
from __future__ import annotations

from contextlib import contextmanager
from pathlib import Path
import tempfile
from typing import Iterator


@contextmanager
def terminal_config(contents: str) -> Iterator[str]:
    with tempfile.TemporaryDirectory(prefix="sopra-terminal-config-") as directory:
        config = Path(directory) / "sopra"
        config.mkdir()
        (config / "config.toml").write_text(contents)
        yield directory
