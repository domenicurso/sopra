#!/usr/bin/env python3
"""Small bidirectional smoke test for the stock-Zsh demo."""

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
from typing import NoReturn

CSI = re.compile(rb"\x1b\[[0-?]*[ -/]*[@-~]")
ROOT = Path(__file__).resolve().parent.parent
START = ROOT / "scripts" / "start-keel.sh"
ROWS = 40
COLUMNS = 100


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
        os.close(master)
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


def wait_for(
    session: tuple[int, int],
    output: bytearray,
    needle: bytes,
    seconds: float,
) -> None:
    master, pid = session
    deadline = time.monotonic() + seconds
    while needle not in output and time.monotonic() < deadline:
        wait = max(0.0, deadline - time.monotonic())
        ready, _, _ = select.select([master], [], [], wait)
        if ready:
            read_available(master, output)
    if needle not in output:
        fail(f"did not see {needle!r}", pid=pid, master=master)


def wait_for_count(
    session: tuple[int, int],
    output: bytearray,
    needle: bytes,
    count: int,
) -> None:
    master, pid = session
    seconds = 3
    deadline = time.monotonic() + seconds
    while output.count(needle) < count and time.monotonic() < deadline:
        wait = max(0.0, deadline - time.monotonic())
        ready, _, _ = select.select([master], [], [], wait)
        if ready:
            read_available(master, output)
    if output.count(needle) < count:
        fail(f"did not see {count} occurrences of {needle!r}", pid=pid, master=master)


def send(master: int, text: bytes) -> None:
    os.write(master, text)


def plain(output: bytes) -> bytes:
    return CSI.sub(b"", output)


def main() -> int:
    env = os.environ.copy()
    env.update({"TERM": "xterm-256color", "COLUMNS": str(COLUMNS), "LINES": str(ROWS)})
    pid, master = pty.fork()
    if pid == 0:
        os.environ.update(env)
        os.execv(str(START), [str(START)])
    session = (master, pid)
    flags = fcntl.fcntl(master, fcntl.F_GETFL)
    fcntl.fcntl(master, fcntl.F_SETFL, flags | os.O_NONBLOCK)
    window = struct.pack("HHHH", ROWS, COLUMNS, 0, 0)
    fcntl.ioctl(master, termios.TIOCSWINSZ, window)
    output = bytearray()

    try:
        wait_for(session, output, b"\x1b[s", 10)
        read_for(master, output, 0.20)
        initial_moves = output.count(b"\x1b[u")
        read_for(master, output, 0.25)
        later_moves = output.count(b"\x1b[u")

        if later_moves <= initial_moves:
            fail("cursor did not trigger another repaint", pid, master)
        if b"\x1b[1;1H" in output:
            fail("renderer used an absolute row-zero cursor move", pid, master)

        rendered = plain(output)
        if b"git status" not in rendered or b"ready" not in rendered:
            fail("initial scene was not rendered", pid, master)

        initial_origins = output.count(b"\x1b[s")
        interrupt_start = len(output)
        send(master, b"git")
        read_for(master, output, 0.08)
        send(master, b"\t")
        read_for(master, output, 0.08)
        send(master, b"\x03")
        wait_for_count(session, output, b"\x1b[s", initial_origins + 1)
        interrupted = plain(output[interrupt_start:])
        if b"\x1b[2m\x1b[38;2;129;142;160m\xe2\x9d\xaf" not in output[interrupt_start:] or b"git status" not in interrupted:
            fail("Ctrl-C did not keep the highlighted transient command", pid, master)

        transient_start = len(output)
        for byte in b"print -r -- keel-transient\r":
            send(master, bytes([byte]))
            read_for(master, output, 0.01)
        wait_for(session, output, b"keel-transient", 3)
        accepted = plain(output[transient_start:])
        if b"\x1b[2m\x1b[38;2;129;142;160m\xe2\x9d\xaf" not in output[transient_start:] or b"print -r --" not in accepted:
            fail("accepted line did not render the compact transient prompt", pid, master)
        if b"\x1b[1m\x1b[38;2;125;196;255mp" not in output[transient_start:]:
            fail("transient command was not painted with the Rust command style", pid, master)
        wait_for_count(session, output, b"\x1b[s", initial_origins + 2)

        escape_start = len(output)
        send(master, b"echo escape-kept")
        read_for(master, output, 0.10)
        send(master, b"\x1b")
        read_for(master, output, 0.10)
        if b"escape-kept" not in plain(output[escape_start:]):
            fail("Escape did not preserve the edited buffer", pid, master)
        send(master, b"\r")
        wait_for(session, output, b"\r\nescape-kept\r\n", 3)
        wait_for_count(session, output, b"\x1b[s", initial_origins + 3)
        send(master, b"\x1b")
        read_for(master, output, 0.10)
        send(master, b"exit\r")
        deadline = time.monotonic() + 3
        while time.monotonic() < deadline:
            waited, status = os.waitpid(pid, os.WNOHANG)
            if waited == pid:
                if os.WIFEXITED(status) and os.WEXITSTATUS(status) == 0:
                    print(
                        "terminal harness: ok "
                        "(scene, cursor repaint, transient interrupt/accept, Escape, shell return)"
                    )
                    os.close(master)
                    return 0
                fail("shell exited unsuccessfully", master=master)
            read_for(master, output, 0.05)
        fail("shell did not exit after the round trip", pid, master)
    except (OSError, select.error) as error:
        fail(str(error), pid, master)


if __name__ == "__main__":
    main()
