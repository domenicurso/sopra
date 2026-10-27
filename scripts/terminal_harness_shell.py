"""PTY checks for shell history and the settings dialog."""
from __future__ import annotations

from terminal_harness_support import (
    FRAME_MARKER, fail, plain, read_for, send, wait_for_count, wait_for_plain_after,
)


def exercise_history_suspension(
    session: tuple[int, int], output: bytearray, origins: int
) -> int:
    command_start = len(output)
    for byte in b"git st\r":
        send(session[0], bytes([byte]))
        read_for(session[0], output, 0.01)
    wait_for_plain_after(session, output, b"gitst", command_start, 3)
    read_for(session[0], output, 0.25)
    origins = output.count(FRAME_MARKER)

    history_start = len(output)
    send(session[0], b"\x1b[A")
    wait_for_plain_after(session, output, b"gitst", history_start, 3)
    read_for(session[0], output, 0.15)
    history_output = plain(output[history_start:])
    if any(marker.encode() in history_output for marker in ("╭", "╮", "╰", "╯")):
        fail("history navigation reopened the completion overlay", *session)

    send(session[0], b"\x1b[D")
    wait_for_plain_after(session, output, b"status", history_start, 3)
    repaint_start = output.count(FRAME_MARKER)
    send(session[0], b"\x03")
    wait_for_count(session, output, FRAME_MARKER, repaint_start + 1)
    return output.count(FRAME_MARKER)


def exercise_settings(session: tuple[int, int], output: bytearray) -> int:
    start = len(output)
    origins = output.count(FRAME_MARKER)
    send(session[0], b"sopra ")
    wait_for_plain_after(session, output, b"settings", start, 3)
    send(session[0], b"\x03")
    wait_for_count(session, output, FRAME_MARKER, origins + 1)
    start = len(output)
    origins = output.count(FRAME_MARKER)
    send(session[0], b"sopra settings\r")
    wait_for_plain_after(session, output, b"Sopra Settings", start, 3)
    wait_for_plain_after(session, output, b"Prompt", start, 3)
    if b"\x1b[?1049h" in output[start:] or b"\x1b[2J" in output[start:]:
        fail("settings cleared the terminal outside its dialog", *session)
    send(session[0], b"\x1b")
    wait_for_count(session, output, FRAME_MARKER, origins + 2)
    return output.count(FRAME_MARKER)


