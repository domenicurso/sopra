#!/usr/bin/env python3
"""Bidirectional PTY smoke test for the stock-Zsh editor."""
from __future__ import annotations
import os
import select
import subprocess
import time
from terminal_harness_support import (
    COLUMNS,
    ROOT,
    ROWS,
    fail,
    plain,
    read_for,
    resize,
    send,
    start_session,
    wait_for,
    wait_for_count,
    wait_for_plain,
    wait_for_plain_after,
)
def check_initial(session: tuple[int, int], output: bytearray) -> int:
    wait_for(session, output, b"\x1b[s", 10)
    wait_for_plain(session, output, b" in ~/", 10)
    read_for(session[0], output, 0.20)
    initial_moves = output.count(b"\x1b[u")
    read_for(session[0], output, 0.25)
    if output.count(b"\x1b[u") <= initial_moves:
        fail("cursor did not trigger another repaint", *session)
    if b"\x1b[1;1H" in output:
        fail("renderer used an absolute row-zero cursor move", *session)
    if b"\x1b[38;2;0;0;0m" not in output or b"\x1b[48;2;0;" not in output:
        fail("cursor did not use the terminal palette response", *session)
    rendered = plain(output)
    if b" in ~/" not in rendered or b"\xe2\x9d\xaf" not in rendered or b"keel-demo" in rendered or b"ready" in rendered:
        fail("product scene was not rendered", *session)
    return output.count(b"\x1b[s")
def exercise_command_completion(session: tuple[int, int], output: bytearray, origins: int) -> int:
    command_start = len(output)
    send(session[0], b"ec")
    wait_for_plain(session, output, b"echo", 3)
    if b"\x1b[31me" not in output[command_start:]:
        fail("fake command was not rendered red", *session)
    green_count = output.count(b"\x1b[32me")
    send(session[0], b"\t")
    wait_for_count(session, output, b"\x1b[32me", green_count + 1)
    send(session[0], b"\x03")
    origins += 1; wait_for_count(session, output, b"\x1b[s", origins)
    option_start = len(output); send(session[0], b"git --vrsn")
    wait_for_plain(session, output, b"--version", 3)
    if b"--version" not in plain(output[option_start:]):
        fail("option completion did not fuzzy-match git flags", *session)
    send(session[0], b"\x03")
    origins += 1; wait_for_count(session, output, b"\x1b[s", origins)
    return origins
def exercise_path_completion(session: tuple[int, int], output: bytearray, origins: int) -> int:
    start = len(output)
    send(session[0], b"cd ")
    wait_for_plain(session, output, b"Cargo.toml", 3)
    if b"ms" not in plain(output[start:]) or b"0.0ms" in plain(output[start:]):
        fail("completion overlay was not rendered", *session)
    start = len(output)
    resize(session[0], 30, 80)
    origins += 1; wait_for_count(session, output, b"\x1b[s", origins)
    if output[start:].count(b"\x1b[2K") < 30:
        fail("resize did not clear the visible terminal rows", *session)
    start = len(output)
    resize(session[0], ROWS, COLUMNS)
    origins += 1; wait_for_count(session, output, b"\x1b[s", origins)
    if output[start:].count(b"\x1b[2K") < ROWS:
        fail("resize did not clear the expanded terminal rows", *session)
    send(session[0], b"\t")
    read_for(session[0], output, 0.10)
    send(session[0], b"\x03")
    origins += 1; wait_for_count(session, output, b"\x1b[s", origins)
    interrupted = plain(output[start:])
    if b"\xe2\x9d\xaf" not in interrupted or b"Cargo.lock" not in interrupted:
        fail("Ctrl-C did not keep the completed transient command", *session)
    return origins

def exercise_nested_help(session: tuple[int, int], output: bytearray, origins: int) -> int:
    start = len(output)
    send(session[0], b"npm install --")
    wait_for_plain_after(session, output, b"--install-strategy", start, 3)
    send(session[0], b"\x03")
    origins += 1
    wait_for_count(session, output, b"\x1b[s", origins)
    return origins
def exercise_accept(session: tuple[int, int], output: bytearray, origins: int) -> int:
    start = len(output)
    for byte in b"print -r -- keel-transient\r":
        send(session[0], bytes([byte]))
        read_for(session[0], output, 0.01)
    wait_for(session, output, b"keel-transient", 3)
    accepted = plain(output[start:])
    if b"\xe2\x9d\xaf" not in accepted or b"print -r --" not in accepted:
        fail("accepted line did not render the transient prompt", *session)
    if b"\x1b[32mp" not in output[start:]:
        fail("accepted line did not keep syntax highlighting", *session)
    origins += 1
    wait_for_count(session, output, b"\x1b[s", origins)
    return origins
def exercise_escape(session: tuple[int, int], output: bytearray, origins: int) -> None:
    start = len(output)
    for byte in b"echo escape-kept":
        send(session[0], bytes([byte]))
        read_for(session[0], output, 0.01)
    wait_for_plain_after(session, output, b"escape-kept", start, 3)
    start = len(output)
    send(session[0], b"\x1b")
    read_for(session[0], output, 0.10)
    if output.count(b"\x1b[s") != origins:
        fail("Escape relaunched the editor loop", *session)
    start = len(output)
    send(session[0], b"\x15cd ")
    wait_for_plain_after(session, output, b"Cargo.toml", start, 3)
    read_for(session[0], output, 0.10)
    start = len(output)
    send(session[0], b"\x1b")
    read_for(session[0], output, 0.10)
    if output.count(b"\x1b[s") != origins:
        fail("Escape relaunched the editor loop", *session)
    if any(marker.encode() in plain(output[start:]) for marker in ("╭", "╮", "─")):
        fail("Escape did not hide the completion overlay", *session)
    start = len(output)
    send(session[0], b"\x15cd ")
    wait_for_plain_after(session, output, b"Cargo.toml", start, 3)
    send(session[0], b"\x03")
    origins += 1
    wait_for_count(session, output, b"\x1b[s", origins)
    send(session[0], b"exit\r")
def wait_for_exit(session: tuple[int, int], output: bytearray) -> None:
    master, pid = session
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        waited, status = os.waitpid(pid, os.WNOHANG)
        if waited == pid:
            if os.WIFEXITED(status) and os.WEXITSTATUS(status) == 0:
                print("terminal harness: ok (PTY scene, palette, completion, transient lines, Escape)")
                os.close(master)
                return
            fail("shell exited unsuccessfully", master=master)
        read_for(master, output, 0.05)
    fail("shell did not exit after the round trip", pid, master)
def main() -> int:
    subprocess.run(
        [str(ROOT / "scripts" / "build-keel.sh")],
        cwd=ROOT,
        check=True,
    )
    session, output = start_session()
    try:
        origins = check_initial(session, output)
        origins = exercise_command_completion(session, output, origins)
        origins = exercise_nested_help(session, output, origins)
        origins = exercise_path_completion(session, output, origins)
        origins = exercise_accept(session, output, origins)
        exercise_escape(session, output, origins)
        wait_for_exit(session, output)
    except (OSError, select.error) as error:
        fail(str(error), *session)
    return 0
if __name__ == "__main__":
    main()
