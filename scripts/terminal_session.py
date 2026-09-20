"""Persistent PTY sessions for interactive, screen-oriented agent testing."""
from __future__ import annotations

import errno
import fcntl
import os
import pty
import select
import signal
import struct
import termios
import time
from collections.abc import Mapping, Sequence

try:
    from .terminal_screen import VirtualTerminal
except ImportError:
    from terminal_screen import VirtualTerminal


KEYS = {
    "ENTER": b"\r", "RETURN": b"\r", "TAB": b"\t", "ESC": b"\x1b",
    "BACKSPACE": b"\x7f", "DELETE": b"\x1b[3~", "UP": b"\x1b[A",
    "DOWN": b"\x1b[B", "RIGHT": b"\x1b[C", "LEFT": b"\x1b[D",
    "HOME": b"\x1b[H", "END": b"\x1b[F", "PAGEUP": b"\x1b[5~",
    "PAGEDOWN": b"\x1b[6~", "INSERT": b"\x1b[2~", "CTRL-C": b"\x03",
    "CTRL-D": b"\x04", "CTRL-Z": b"\x1a", "CTRL-L": b"\x0c",
    "CTRL-A": b"\x01", "CTRL-E": b"\x05", "CTRL-U": b"\x15",
    "CTRL-K": b"\x0b", "CTRL-W": b"\x17",
}


class TerminalSession:
    """Own one child process, its PTY, and the screen accumulated across turns."""

    def __init__(
        self,
        command: Sequence[str],
        *,
        rows: int = 40,
        columns: int = 100,
        cwd: str | None = None,
        env: Mapping[str, str] | None = None,
    ) -> None:
        if not command:
            raise ValueError("command must contain an executable")
        self.command = tuple(command)
        self.screen = VirtualTerminal(rows, columns)
        self.output = bytearray()
        self.returncode: int | None = None
        self._closed = False
        child_env = os.environ.copy()
        child_env.update({"TERM": "xterm-256color", "COLORTERM": "truecolor"})
        if env is not None:
            child_env.update(env)
        child_env.update({"COLUMNS": str(columns), "LINES": str(rows)})
        self._pid, self._master = pty.fork()
        if self._pid == 0:
            self._run_child(child_env, cwd)
        flags = fcntl.fcntl(self._master, fcntl.F_GETFL)
        fcntl.fcntl(self._master, fcntl.F_SETFL, flags | os.O_NONBLOCK)
        self.resize(rows, columns)

    def _run_child(self, environment: Mapping[str, str], cwd: str | None) -> None:
        try:
            if cwd is not None:
                os.chdir(cwd)
            os.execvpe(self.command[0], list(self.command), dict(environment))
        except OSError as error:
            os.write(2, f"terminal session: {error}\n".encode())
            os._exit(127)

    def send(self, data: bytes | str) -> None:
        payload = data.encode() if isinstance(data, str) else data
        while payload:
            try:
                written = os.write(self._master, payload)
            except InterruptedError:
                continue
            payload = payload[written:]

    def send_text(self, text: str) -> None:
        self.send(text)

    def paste(self, text: str) -> None:
        self.send(b"\x1b[200~" + text.encode() + b"\x1b[201~")

    def key(self, key: str) -> None:
        normalized = key.upper()
        if normalized in KEYS:
            self.send(KEYS[normalized])
        elif len(key) == 1:
            self.send(key)
        else:
            raise KeyError(f"unknown terminal key: {key}")

    def read(self, timeout: float = 0.05) -> bytes:
        deadline = time.monotonic() + max(0.0, timeout)
        received = bytearray()
        while True:
            remaining = max(0.0, deadline - time.monotonic())
            ready, _, _ = select.select([self._master], [], [], remaining)
            if not ready:
                break
            try:
                chunk = os.read(self._master, 65_536)
            except OSError as error:
                if error.errno in (errno.EAGAIN, errno.EWOULDBLOCK, errno.EIO):
                    break
                raise
            if not chunk:
                break
            received.extend(chunk)
            self.output.extend(chunk)
            for response in self.screen.feed(chunk):
                self.send(response)
            if timeout <= 0:
                continue
        self._poll_exit()
        return bytes(received)

    def wait_for(self, text: str, timeout: float = 3.0) -> None:
        deadline = time.monotonic() + timeout
        while text not in self.screen.text(trim=False):
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError(f"terminal did not render {text!r}\n{self.screen.text()}")
            self.read(min(0.05, remaining))

    def resize(self, rows: int, columns: int) -> None:
        size = struct.pack("HHHH", max(1, rows), max(1, columns), 0, 0)
        fcntl.ioctl(self._master, termios.TIOCSWINSZ, size)
        self.screen.resize(rows, columns)

    def _poll_exit(self) -> bool:
        if self.returncode is not None:
            return False
        waited, status = os.waitpid(self._pid, os.WNOHANG)
        if waited != self._pid:
            return True
        self.returncode = os.waitstatus_to_exitcode(status)
        return False

    @property
    def alive(self) -> bool:
        return self._poll_exit()

    def _signal_process_group(self, signal_number: int) -> None:
        try:
            process_group = os.getpgid(self._pid)
            if process_group == os.getpgrp():
                os.kill(self._pid, signal_number)
            else:
                os.killpg(process_group, signal_number)
        except ProcessLookupError:
            pass

    def close(self) -> None:
        if self._closed:
            return
        if self.returncode is None:
            self._signal_process_group(signal.SIGTERM)
            deadline = time.monotonic() + 0.5
            while self.alive and time.monotonic() < deadline:
                time.sleep(0.01)
            if self.alive:
                self._signal_process_group(signal.SIGKILL)
                deadline = time.monotonic() + 0.5
                while self.alive and time.monotonic() < deadline:
                    time.sleep(0.01)
                if self.returncode is None:
                    self.returncode = -signal.SIGKILL
        os.close(self._master)
        self._closed = True

    def __enter__(self) -> "TerminalSession":
        return self

    def __exit__(self, *_: object) -> None:
        self.close()
