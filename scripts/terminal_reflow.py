"""A native primary-screen terminal model shared by interactive regressions."""
from __future__ import annotations

import base64
import json
import subprocess
from typing import TypedDict, cast


class Snapshot(TypedDict):
    buffer: str
    viewport: int
    mouseTracking: str
    bracketedPaste: bool
    responses: list[str]
    lines: list[str]
    base: int
    cursor: list[int]
    paintedCursors: list[list[int]]


class ReflowScreen:
    def __init__(self, process: subprocess.Popen[str]) -> None:
        self.process = process
        self.rows, self.columns = 12, 120

    def request(self, payload: dict[str, object]) -> Snapshot:
        assert self.process.stdin is not None and self.process.stdout is not None
        self.process.stdin.write(json.dumps(payload) + "\n")
        self.process.stdin.flush()
        return cast(Snapshot, json.loads(self.process.stdout.readline()))

    def feed(self, data: bytes) -> tuple[bytes, ...]:
        result = self.request({"op": "feed", "data": base64.b64encode(data).decode()})
        return tuple(base64.b64decode(response) for response in result["responses"])

    def resize(self, rows: int, columns: int) -> None:
        self.rows, self.columns = rows, columns
        self.request({"op": "resize", "rows": rows, "columns": columns})

    def snapshot(self) -> Snapshot:
        return self.request({"op": "snapshot"})

    def text(self, trim: bool = True) -> str:
        snapshot = self.snapshot()
        return "\n".join(snapshot["lines"][snapshot["base"] : snapshot["base"] + self.rows])

