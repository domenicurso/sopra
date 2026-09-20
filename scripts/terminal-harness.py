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
    if b" in ~/" not in rendered or b"$" not in rendered or b"sopra-demo" in rendered or b"ready" in rendered:
        fail("product scene was not rendered", *session)
    return output.count(b"\x1b[s")
def exercise_command_completion(session: tuple[int, int], output: bytearray, origins: int) -> int:
    command_start = len(output)
    send(session[0], b"ech")
    wait_for_plain(session, output, b"echo", 3)
    if b"\x1b[31me" not in output[command_start:]:
        fail("fake command was not rendered red", *session)
    green_count = output.count(b"\x1b[32me")
    send(session[0], b"\t")
    wait_for_count(session, output, b"\x1b[32me", green_count + 1)
    send(session[0], b"\x03")
    origins += 1; wait_for_count(session, output, b"\x1b[s", origins)
    history_start = len(output)
    send(session[0], b"git ")
    wait_for_plain_after(session, output, b"0/", history_start, 3)
    send(session[0], b"\x1b[A")
    wait_for_plain_after(session, output, b"1/", history_start, 3)
    send(session[0], b"\x03")
    origins += 1; wait_for_count(session, output, b"\x1b[s", origins)
    option_start = len(output); send(session[0], b"git --vrsn")
    wait_for_plain(session, output, b"version", 3)
    if b"version" not in plain(output[option_start:]):
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
    repaint_start = output.count(b"\x1b[s")
    resize(session[0], 30, 80)
    wait_for_count(session, output, b"\x1b[s", repaint_start + 1)
    origins = output.count(b"\x1b[s")
    if output[start:].count(b"\x1b[2K") < 30:
        fail("resize did not clear the visible terminal rows", *session)
    start = len(output)
    repaint_start = output.count(b"\x1b[s")
    resize(session[0], ROWS, COLUMNS)
    wait_for_count(session, output, b"\x1b[s", repaint_start + 1)
    origins = output.count(b"\x1b[s")
    if output[start:].count(b"\x1b[2K") < ROWS:
        fail("resize did not clear the expanded terminal rows", *session)
    send(session[0], b"\t")
    read_for(session[0], output, 0.10)
    send(session[0], b"\x03")
    repaint_start = output.count(b"\x1b[s")
    wait_for_count(session, output, b"\x1b[s", repaint_start + 1)
    origins = output.count(b"\x1b[s")
    interrupted = plain(output[start:])
    if b"$" not in interrupted or b"Cargo.lock" not in interrupted:
        fail("Ctrl-C did not keep the completed transient command", *session)
    return origins

def exercise_nested_help(session: tuple[int, int], output: bytearray, origins: int) -> int:
    start = len(output)
    send(session[0], b"cargo build --prof")
    wait_for_plain_after(session, output, b"--profile", start, 3)
    send(session[0], b"\x03")
    origins += 1
    wait_for_count(session, output, b"\x1b[s", origins)
    return origins
def exercise_accept(session: tuple[int, int], output: bytearray, origins: int) -> int:
    start = len(output)
    for byte in b"print -r -- sopra-transient\r":
        send(session[0], bytes([byte]))
        read_for(session[0], output, 0.01)
    wait_for(session, output, b"sopra-transient", 3)
    accepted = plain(output[start:])
    if b"$" not in accepted or b"print -r --" not in accepted:
        fail("accepted line did not render the transient prompt", *session)
    if b"\x1b[32mp" not in output[start:]:
        fail("accepted line did not keep syntax highlighting", *session)
    if b"\x1b[1m\x1b[32mprint" not in output[start:]:
        fail("shell re-rendered the accepted command without syntax highlighting", *session)
    clear_start = len(output)
    send(session[0], b"clear\r")
    wait_for(session, output, b"\x1b[2J", 3)
    if b"\x1b[2J" not in output[clear_start:]:
        fail("clear did not reach the shell", *session)
    wait_for_count(session, output, b"\x1b[s", origins)
    read_for(session[0], output, 0.20)
    return output.count(b"\x1b[s")


def exercise_history_suspension(
    session: tuple[int, int], output: bytearray, origins: int
) -> int:
    command_start = len(output)
    for byte in b"git st\r":
        send(session[0], bytes([byte]))
        read_for(session[0], output, 0.01)
    wait_for_plain_after(session, output, b"gitst", command_start, 3)
    read_for(session[0], output, 0.25)
    origins = output.count(b"\x1b[s")

    history_start = len(output)
    send(session[0], b"\x1b[A")
    wait_for_plain_after(session, output, b"gitst", history_start, 3)
    read_for(session[0], output, 0.15)
    history_output = plain(output[history_start:])
    if any(marker.encode() in history_output for marker in ("╭", "╮", "╰", "╯")):
        fail("history navigation reopened the completion overlay", *session)

    send(session[0], b"\x1b[D")
    wait_for_plain_after(session, output, b"status", history_start, 3)
    repaint_start = output.count(b"\x1b[s")
    send(session[0], b"\x03")
    wait_for_count(session, output, b"\x1b[s", repaint_start + 1)
    return output.count(b"\x1b[s")


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
    send(session[0], b"exit 0\r")
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
        [str(ROOT / "scripts" / "build.sh")],
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
        origins = exercise_history_suspension(session, output, origins)
        exercise_escape(session, output, origins)
        wait_for_exit(session, output)
    except (OSError, select.error) as error:
        fail(str(error), *session)
    return 0
if __name__ == "__main__":
    main()
