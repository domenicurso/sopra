"""Shared PTY and ANSI helpers for the terminal harness."""
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

CSI = re.compile(rb"\x1b\[[0-?]*[ -/]*[@-~]")
ROOT = Path(__file__).resolve().parent.parent
START = ROOT / "scripts" / "start.sh"
ROWS, COLUMNS = 40, 100
ASYNC_COMPLETION_TIMEOUT = 10


def fail(message: str, pid: int | None = None, master: int | None = None) -> NoReturn:
    if pid is not None:
        try:
            os.kill(pid, signal.SIGKILL)
        except (PermissionError, ProcessLookupError):
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
        try:
            os.close(master)
        except OSError:
            pass
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


def wait_until(
    session: tuple[int, int], output: bytearray, done: Callable[[], bool], seconds: float
) -> bool:
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


def wait_for_plain(
    session: tuple[int, int], output: bytearray, needle: bytes, seconds: float
) -> None:
    if not wait_until(
        session,
        output,
        lambda: has_plain(output, needle),
        max(seconds, ASYNC_COMPLETION_TIMEOUT),
    ):
        fail(
            f"did not see rendered {needle!r}; output tail={plain(output)[-500:]!r}",
            *session,
        )


def wait_for_plain_after(
    session: tuple[int, int], output: bytearray, needle: bytes, start: int, seconds: float
) -> None:
    if not wait_until(
        session,
        output,
        lambda: has_plain(output[start:], needle),
        max(seconds, ASYNC_COMPLETION_TIMEOUT),
    ):
        fail(
            f"did not see rendered {needle!r} after the current action; "
            f"output tail={plain(output[start:])[-500:]!r}",
            *session,
        )


def wait_for_count(session: tuple[int, int], output: bytearray, needle: bytes, count: int) -> None:
    if not wait_until(session, output, lambda: output.count(needle) >= count, 3):
        fail(f"did not see {count} occurrences of {needle!r}", *session)


def send(master: int, text: bytes) -> None:
    os.write(master, text)


def resize(master: int, rows: int, columns: int) -> None:
    window = struct.pack("HHHH", rows, columns, 0, 0)
    fcntl.ioctl(master, termios.TIOCSWINSZ, window)


def plain(output: bytes) -> bytes:
    return CSI.sub(b"", output)


def has_plain(output: bytes, needle: bytes) -> bool:
    rendered = plain(output)
    return needle in rendered or needle in rendered.replace(b" ", b"")


def start_session() -> tuple[tuple[int, int], bytearray]:
    env = os.environ.copy()
    env.update({"TERM": "xterm-256color", "COLUMNS": str(COLUMNS), "LINES": str(ROWS)})
    pid, master = pty.fork()
    if pid == 0:
        os.environ.update(env)
        os.execv(str(START), [str(START)])
    flags = fcntl.fcntl(master, fcntl.F_GETFL)
    fcntl.fcntl(master, fcntl.F_SETFL, flags | os.O_NONBLOCK)
    resize(master, ROWS, COLUMNS)
    return (master, pid), bytearray()
