#!/usr/bin/env python3
"""Foreground terminal applications, input routing, signals, and cleanup."""
from __future__ import annotations

import os
import signal
import sys
import time

from terminal_host import scroll_up, hosted, submit, wait_until


PROGRAM = '''import os, sys, termios, tty
fd = sys.stdin.fileno()
saved = termios.tcgetattr(fd)
try:
    tty.setraw(fd)
    os.write(1, b"\\x1b[?1049h\\x1b[?1002h\\x1b[?1006h\\x1b[HAPP-READY")
    data = b""
    while not data.endswith(b"q"):
        data += os.read(fd, 64)
    open("received", "wb").write(data)
finally:
    os.write(1, b"\\x1b[?1002l\\x1b[?1006l\\x1b[?1049l")
    termios.tcsetattr(fd, termios.TCSANOW, saved)
'''


def test_foreground_program() -> None:
    with hosted() as (terminal, root):
        (root / "app.py").write_text(PROGRAM)
        submit(terminal, f"'{sys.executable}' app.py")
        terminal.wait_for("APP-READY")
        terminal.send(b"\x1b[<64;12;8M")
        terminal.send(b"q")
        wait_until(terminal, lambda: (root / "received").exists())
        assert (root / "received").read_bytes() == b"\x1b[<64;12;8Mq"
        terminal.read(0.3)
        assert "rprompt" in terminal.screen.text(), terminal.screen.text()
        submit(terminal, "sleep 30")
        terminal.key("CTRL-C")
        terminal.read(0.4)
        submit(terminal, "touch interrupted")
        wait_until(terminal, lambda: (root / "interrupted").exists())



def test_shutdown() -> None:
    with hosted() as (terminal, root):
        submit(terminal, "sleep 30 & print $! > child.pid; wait")
        child = int((root / "child.pid").read_text())
        os.kill(terminal._pid, signal.SIGTERM)
        wait_until(terminal, lambda: not terminal.alive)
        assert terminal.returncode == 143
        assert b"\x1b[?1006l" in terminal.output and b"\x1b[?1049l" in terminal.output
        deadline = time.monotonic() + 2
        while time.monotonic() < deadline:
            try:
                os.kill(child, 0)
            except ProcessLookupError:
                break
            time.sleep(0.02)
        else:
            raise AssertionError(f"foreground child {child} survived host shutdown")


def test_literal_paste_and_long_output() -> None:
    with hosted() as (terminal, root):
        terminal.paste("printf 'pasted-one\\n'\nprintf 'pasted-two\\n'; touch submitted")
        terminal.read(0.3)
        assert not (root / "submitted").exists()
        assert "pasted-one" in terminal.screen.text() and "pasted-two" in terminal.screen.text()
        terminal.key("ENTER")
        wait_until(terminal, lambda: (root / "submitted").exists())
        terminal.read(0.1)
        submit(terminal, "for i in {1..1000}; do print row-$i; done")
        scroll_up(terminal)
        assert "row-990" in terminal.screen.text(), terminal.screen.text()
        assert "rprompt" not in terminal.screen.text(), terminal.screen.text()



def test_duplex_backpressure() -> None:
    with hosted() as (terminal, root):
        program = PROGRAM[:PROGRAM.index("try:")] + """try:
    tty.setraw(fd)
    os.write(1, b'DUPLEX-READY')
    data = os.read(fd, 1024)
    os.write(1, b'x' * 262144)
    while len(data) < 262144:
        data += os.read(fd, 65536)
    open('received', 'wb').write(data)
finally:
    termios.tcsetattr(fd, termios.TCSANOW, saved)
"""
        (root / "duplex.py").write_text(program)
        submit(terminal, f"'{sys.executable}' duplex.py")
        terminal.wait_for("DUPLEX-READY")
        payload = memoryview(b"\x1b[200~" + b"k" * 262144 + b"\x1b[201~")
        deadline = time.monotonic() + 10
        while payload:
            assert time.monotonic() < deadline, "duplex transfer stalled"
            try:
                payload = payload[os.write(terminal._master, payload[:4096]):]
            except BlockingIOError:
                pass
            terminal.read(0.005)
        wait_until(terminal, lambda: (root / "received").exists(), timeout=10)
        assert (root / "received").read_bytes() == b"k" * 262144


if __name__ == "__main__":
    test_foreground_program()
    test_shutdown()
    test_literal_paste_and_long_output()
    test_duplex_backpressure()
    print("Hosted foreground mouse routing, interrupts, and shutdown passed")
