#!/usr/bin/env python3
"""JSON-lines controller for a persistent TerminalSession."""
from __future__ import annotations

import argparse
import base64
import json
import sys
from collections.abc import Sequence

from terminal_session import TerminalSession


def snapshot(session: TerminalSession) -> dict[str, object]:
    screen = session.screen
    return {
        "rows": screen.rows,
        "columns": screen.columns,
        "cursor": {"row": screen.row, "column": screen.column},
        "cursor_visible": screen.cursor_visible,
        "lines": screen.lines(),
        "alive": session.alive,
    }


def response(
    session: TerminalSession,
    operation: str,
    output: bytes = b"",
    include_output: bool = False,
) -> None:
    payload = {"ok": True, "op": operation}
    if include_output:
        payload["output_base64"] = base64.b64encode(output).decode()
    payload["screen"] = snapshot(session)
    print(json.dumps(payload, ensure_ascii=False), flush=True)


def error(operation: str, message: str) -> None:
    print(json.dumps({"ok": False, "op": operation, "error": message}), flush=True)


def handle(session: TerminalSession, request: dict[str, object]) -> bool:
    operation = str(request.get("op", ""))
    settle = float(request.get("settle", 0.05))
    include_output = request.get("include_output", False)
    if not isinstance(include_output, bool):
        raise ValueError("include_output must be a boolean")
    if operation == "send":
        session.send_text(str(request.get("data", "")))
        response(session, operation, session.read(settle), include_output)
    elif operation == "paste":
        session.paste(str(request.get("data", "")))
        response(session, operation, session.read(settle), include_output)
    elif operation == "key":
        session.key(str(request["key"]))
        response(session, operation, session.read(settle), include_output)
    elif operation == "read":
        response(session, operation, session.read(float(request.get("timeout", 0.05))), include_output)
    elif operation == "wait_for":
        output_start = len(session.output)
        session.wait_for(str(request["text"]), float(request.get("timeout", 3.0)))
        output = bytes(session.output[output_start:])
        response(session, operation, output, include_output)
    elif operation == "resize":
        session.resize(int(request["rows"]), int(request["columns"]))
        response(session, operation, include_output=include_output)
    elif operation == "screen":
        response(session, operation, include_output=include_output)
    elif operation == "close":
        session.close()
        response(session, operation, include_output=include_output)
        return False
    else:
        raise ValueError(f"unknown operation: {operation}")
    return True


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Drive one persistent PTY and read its virtual screen.")
    parser.add_argument("--rows", type=int, default=40)
    parser.add_argument("--columns", type=int, default=100)
    parser.add_argument("command", nargs=argparse.REMAINDER, help="command after --")
    args = parser.parse_args(argv)
    if args.command[:1] == ["--"]:
        args.command = args.command[1:]
    if not args.command:
        parser.error("a command is required after --")
    return args


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    with TerminalSession(args.command, rows=args.rows, columns=args.columns) as session:
        for line in sys.stdin:
            if not line.strip():
                continue
            try:
                request = json.loads(line)
                if not isinstance(request, dict):
                    raise ValueError("request must be a JSON object")
                if not handle(session, request):
                    break
            except Exception as exc:  # The controller reports malformed turns and stays usable.
                operation = "unknown"
                try:
                    operation = str(request.get("op", "unknown"))
                except (NameError, AttributeError):
                    pass
                error(operation, str(exc))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
