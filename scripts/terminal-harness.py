#!/usr/bin/env python3
"""Bidirectional PTY smoke test for the stock-Zsh editor."""
from __future__ import annotations
import errno
import fcntl
import os
import pty
import re
import select
import signal
import struct
import termios
import time
from pathlib import Path
from typing import Callable, NoReturn
CSI = re.compile(rb"\x1b\[[0-?]*[ -/]*[@-~]"); ROOT = Path(__file__).resolve().parent.parent
START = ROOT / "scripts" / "start-keel.sh"; ROWS, COLUMNS = 40, 100
def fail(message: str, pid: int | None = None, master: int | None = None) -> NoReturn:
    if pid is not None:
        try:
            os.kill(pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        deadline = time.monotonic() + 0.5
        while time.monotonic() < deadline:
            try:
                waited, _ = os.waitpid(pid, os.WNOHANG)
            except ChildProcessError:
                break
            if waited == pid:
                break
            time.sleep(0.01)
    if master is not None:
        try: os.close(master)
        except OSError: pass
    raise SystemExit(f"terminal harness: {message}")
def read_available(master: int, output: bytearray) -> None:
    while True:
        ready, _, _ = select.select([master], [], [], 0)
        if not ready:
            return
        try:
            chunk = os.read(master, 65_536)
            if not chunk:
                return
            output.extend(chunk)
            if b"\x1b]11;?\x1b\\" in chunk:
                os.write(
                    master,
                    b"\x1b]11;rgb:0000/0000/0000\x1b\\"
                    b"\x1b]12;#00ff00\x07",
                )
        except OSError as error:
            if error.errno in (errno.EAGAIN, errno.EWOULDBLOCK, errno.EIO):
                return
            raise
def read_for(master: int, output: bytearray, seconds: float) -> None:
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        wait = max(0.0, deadline - time.monotonic())
        ready, _, _ = select.select([master], [], [], wait)
        if ready:
            read_available(master, output)
def wait_until(session: tuple[int, int], output: bytearray, done: Callable[[], bool], seconds: float) -> bool:
    master, _ = session
    deadline = time.monotonic() + seconds
    while not done() and time.monotonic() < deadline:
        wait = max(0.0, deadline - time.monotonic())
        ready, _, _ = select.select([master], [], [], wait)
        if ready:
            read_available(master, output)
    return done()
def wait_for(session: tuple[int, int], output: bytearray, needle: bytes, seconds: float) -> None:
    if not wait_until(session, output, lambda: needle in output, seconds):
        fail(f"did not see {needle!r}", *session)
def wait_for_plain(session: tuple[int, int], output: bytearray, needle: bytes, seconds: float) -> None:
    if not wait_until(session, output, lambda: needle in plain(output), seconds):
        fail(f"did not see rendered {needle!r}", *session)
def wait_for_count(session: tuple[int, int], output: bytearray, needle: bytes, count: int) -> None:
    if not wait_until(session, output, lambda: output.count(needle) >= count, 3):
        fail(f"did not see {count} occurrences of {needle!r}", *session)
def send(master: int, text: bytes) -> None: os.write(master, text)
def plain(output: bytes) -> bytes:
    return CSI.sub(b"", output)
def start_session() -> tuple[tuple[int, int], bytearray]:
    env = os.environ.copy()
    env.update({"TERM": "xterm-256color", "COLUMNS": str(COLUMNS), "LINES": str(ROWS)})
    pid, master = pty.fork()
    if pid == 0:
        os.environ.update(env)
        os.execv(str(START), [str(START)])
    flags = fcntl.fcntl(master, fcntl.F_GETFL)
    fcntl.fcntl(master, fcntl.F_SETFL, flags | os.O_NONBLOCK)
    window = struct.pack("HHHH", ROWS, COLUMNS, 0, 0)
    fcntl.ioctl(master, termios.TIOCSWINSZ, window)
    return (master, pid), bytearray()
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
    if b"Keel" not in plain(output[start:]):
        fail("completion overlay was not rendered", *session)
    send(session[0], b"\t")
    read_for(session[0], output, 0.10)
    send(session[0], b"\x03")
    origins += 1; wait_for_count(session, output, b"\x1b[s", origins)
    interrupted = plain(output[start:])
    if b"\xe2\x9d\xaf" not in interrupted or b"Cargo.lock" not in interrupted:
        fail("Ctrl-C did not keep the completed transient command", *session)
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
    send(session[0], b"echo escape-kept")
    read_for(session[0], output, 0.10)
    send(session[0], b"\x1b")
    read_for(session[0], output, 0.10)
    if b"escape-kept" not in plain(output[start:]):
        fail("Escape did not preserve the edited buffer", *session)
    send(session[0], b"\r")
    wait_for(session, output, b"\r\nescape-kept\r\n", 3)
    origins += 1
    wait_for_count(session, output, b"\x1b[s", origins)
    send(session[0], b"\x1b")
    read_for(session[0], output, 0.10)
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
    session, output = start_session()
    try:
        origins = check_initial(session, output)
        origins = exercise_command_completion(session, output, origins)
        origins = exercise_path_completion(session, output, origins)
        origins = exercise_accept(session, output, origins)
        exercise_escape(session, output, origins)
        wait_for_exit(session, output)
    except (OSError, select.error) as error:
        fail(str(error), *session)
    return 0
if __name__ == "__main__":
    main()
